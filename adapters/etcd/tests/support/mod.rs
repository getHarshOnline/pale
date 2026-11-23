//! Owned native etcd quorum, test-only PKI and bounded HTTPS administration.

pub(crate) mod proxy;

use std::{
    fs,
    net::TcpListener,
    num::NonZeroU64,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use pale_adapter_etcd::EtcdAnchorStore;
use pale_core::records::ArtifactDigest;
use reqwest::{blocking::Client, Certificate, Identity};
use serde_json::{json, Value};

pub(crate) struct Quorum {
    pub(crate) root: tempfile::TempDir,
    binary: PathBuf,
    clients: Vec<u16>,
    peers: Vec<u16>,
    children: Vec<Option<Child>>,
    pub(crate) client: Client,
    pub(crate) token: String,
    pub(crate) cluster: NonZeroU64,
    pub(crate) ca: Vec<u8>,
    pub(crate) identity: Vec<u8>,
}

impl Quorum {
    pub(crate) fn start() -> Self {
        let input =
            std::env::var_os("PALE_ETCD_TEST_BINARY").expect("select the native etcd binary");
        let expected =
            std::env::var("PALE_ETCD_TEST_BINARY_SHA256").expect("select its verified SHA-256");
        let bytes = fs::read(input).unwrap();
        assert_eq!(ArtifactDigest::hash_bytes(&bytes).as_str(), expected);
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let binary = root.path().join("etcd");
        fs::write(&binary, bytes).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        let version = Command::new(&binary).arg("--version").output().unwrap();
        assert!(version.status.success());
        assert!(String::from_utf8(version.stdout)
            .unwrap()
            .contains("etcd Version: 3.6.16"));
        pki(root.path());
        let ca = fs::read(root.path().join("ca.crt")).unwrap();
        let identity = [
            fs::read(root.path().join("client.crt")).unwrap(),
            fs::read(root.path().join("client.key")).unwrap(),
        ]
        .concat();
        let client = Client::builder()
            .tls_certs_only([Certificate::from_pem(&ca).unwrap()])
            .identity(Identity::from_pem(&identity).unwrap())
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let mut result = Self {
            root,
            binary,
            clients: (0..3).map(|_| port()).collect(),
            peers: (0..3).map(|_| port()).collect(),
            children: vec![None, None, None],
            client,
            token: String::new(),
            cluster: NonZeroU64::new(1).unwrap(),
            ca,
            identity,
        };
        for index in 0..3 {
            result.restart(index);
        }
        result.wait_ready(0);
        let status = result.post(0, "/v3/maintenance/status", json!({}));
        result.cluster = NonZeroU64::new(
            status["header"]["cluster_id"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
        )
        .unwrap();
        result.post(
            0,
            "/v3/auth/user/add",
            json!({"name":"root","password":"fixture-only-password"}),
        );
        result.post(0, "/v3/auth/role/add", json!({"name":"root"}));
        result.post(
            0,
            "/v3/auth/user/grant",
            json!({"user":"root","role":"root"}),
        );
        result.post(0, "/v3/auth/enable", json!({}));
        result.refresh_token();
        result
    }

    pub(crate) fn refresh_token(&mut self) {
        let reply = self
            .client
            .post(format!("{}/v3/auth/authenticate", self.endpoint(0)))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(
                serde_json::to_vec(&json!({"name":"root","password":"fixture-only-password"}))
                    .unwrap(),
            )
            .send()
            .unwrap();
        let status = reply.status();
        let bytes = reply.bytes().unwrap();
        assert!(
            status.is_success(),
            "native fixture authentication: {status}: {}",
            String::from_utf8_lossy(&bytes)
        );
        self.token = serde_json::from_slice::<Value>(&bytes).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_owned();
    }

    pub(crate) fn endpoint(&self, index: usize) -> String {
        format!("https://127.0.0.1:{}", self.clients[index])
    }

    pub(crate) fn store(&self, index: usize) -> EtcdAnchorStore {
        self.store_at(&self.endpoint(index), self.cluster, &self.token)
    }

    pub(crate) fn store_at(
        &self,
        endpoint: &str,
        cluster: NonZeroU64,
        token: &str,
    ) -> EtcdAnchorStore {
        EtcdAnchorStore::new(
            endpoint,
            cluster,
            b"/pale-test/",
            &self.ca,
            token,
            Some(&self.identity),
            Duration::from_secs(3),
        )
        .unwrap()
    }

    pub(crate) fn post(&self, index: usize, path: &str, body: Value) -> Value {
        let mut request = self
            .client
            .post(format!("{}{path}", self.endpoint(index)))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_vec(&body).unwrap());
        if !self.token.is_empty() {
            request = request.header(reqwest::header::AUTHORIZATION, &self.token);
        }
        let response = request.send().unwrap();
        assert!(
            response.status().is_success(),
            "native fixture administration failed: {}",
            response.status()
        );
        serde_json::from_reader(response).unwrap()
    }

    pub(crate) fn stop(&mut self, index: usize) {
        if let Some(mut child) = self.children[index].take() {
            child.kill().unwrap();
            child.wait().unwrap();
        }
    }

    pub(crate) fn restart(&mut self, index: usize) {
        assert!(self.children[index].is_none());
        let cluster = (0..3)
            .map(|i| format!("member{i}=https://127.0.0.1:{}", self.peers[i]))
            .collect::<Vec<_>>()
            .join(",");
        let client = self.endpoint(index);
        let peer = format!("https://127.0.0.1:{}", self.peers[index]);
        let data = self.root.path().join(format!("data{index}"));
        let cert = self.root.path().join("server.crt");
        let key = self.root.path().join("server.key");
        let ca = self.root.path().join("ca.crt");
        let log = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.root.path().join(format!("member{index}.log")))
            .unwrap();
        let child = Command::new(&self.binary)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.root.path())
            .arg("--name")
            .arg(format!("member{index}"))
            .arg("--data-dir")
            .arg(data)
            .arg("--listen-client-urls")
            .arg(&client)
            .arg("--advertise-client-urls")
            .arg(&client)
            .arg("--listen-peer-urls")
            .arg(&peer)
            .arg("--initial-advertise-peer-urls")
            .arg(&peer)
            .arg("--initial-cluster")
            .arg(cluster)
            .arg("--initial-cluster-token")
            .arg("owned-qualification-only")
            .arg("--cert-file")
            .arg(&cert)
            .arg("--key-file")
            .arg(&key)
            .arg("--trusted-ca-file")
            .arg(&ca)
            .arg("--client-cert-auth=true")
            .arg("--peer-cert-file")
            .arg(&cert)
            .arg("--peer-key-file")
            .arg(&key)
            .arg("--peer-trusted-ca-file")
            .arg(&ca)
            .arg("--peer-client-cert-auth=true")
            .arg("--log-level")
            .arg("error")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .unwrap();
        self.children[index] = Some(child);
    }

    pub(crate) fn wait_ready(&mut self, index: usize) {
        let end = Instant::now() + Duration::from_secs(25);
        while Instant::now() < end {
            assert!(
                self.children[index]
                    .as_mut()
                    .unwrap()
                    .try_wait()
                    .unwrap()
                    .is_none(),
                "native etcd fixture exited before readiness"
            );
            if self
                .client
                .get(format!("{}/health", self.endpoint(index)))
                .send()
                .is_ok_and(|r| r.status().is_success())
            {
                return;
            }
            thread::sleep(Duration::from_millis(40));
        }
        panic!("owned native quorum did not become ready");
    }
}

impl Drop for Quorum {
    fn drop(&mut self) {
        for child in &mut self.children {
            if let Some(mut child) = child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn pki(root: &Path) {
    let extension = root.join("extensions.cnf");
    fs::write(&extension,"basicConstraints=critical,CA:FALSE\nsubjectAltName=IP:127.0.0.1,DNS:localhost\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth,clientAuth\n").unwrap();
    let run = |args: &[&str]| {
        let output = Command::new("openssl")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success(), "native test PKI command failed");
    };
    run(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-sha256",
        "-days",
        "1",
        "-subj",
        "/CN=Owned etcd test CA",
        "-addext",
        "basicConstraints=critical,CA:TRUE",
        "-keyout",
        "ca.key",
        "-out",
        "ca.crt",
    ]);
    run(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-subj",
        "/CN=Owned etcd test member",
        "-keyout",
        "server.key",
        "-out",
        "server.csr",
    ]);
    run(&[
        "x509",
        "-req",
        "-in",
        "server.csr",
        "-CA",
        "ca.crt",
        "-CAkey",
        "ca.key",
        "-CAcreateserial",
        "-days",
        "1",
        "-sha256",
        "-extfile",
        "extensions.cnf",
        "-out",
        "server.crt",
    ]);
    // The JSON gateway rejects a client certificate with a nonempty CN when
    // account authentication is enabled. mTLS still authenticates transport;
    // the separately supplied token authenticates the native etcd account.
    fs::write(root.join("client-extensions.cnf"),
        "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=clientAuth\n").unwrap();
    run(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-subj",
        "/O=Owned etcd test client",
        "-keyout",
        "client.key",
        "-out",
        "client.csr",
    ]);
    run(&[
        "x509",
        "-req",
        "-in",
        "client.csr",
        "-CA",
        "ca.crt",
        "-CAkey",
        "ca.key",
        "-CAcreateserial",
        "-days",
        "1",
        "-sha256",
        "-extfile",
        "client-extensions.cnf",
        "-out",
        "client.crt",
    ]);
}

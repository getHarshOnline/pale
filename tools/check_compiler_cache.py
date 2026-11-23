#!/usr/bin/env python3
"""Qualify local compiler-result reuse while keeping Cargo targets separate.

Run against an explicitly selected sccache binary. The disposable fixture has
identical package names and versions in separate checkouts, old source
timestamps and different source contents. Their path-source identities differ.
It also proves reuse of one unchanged shared dependency and invalidation after
its include_str! input changes. This is not a qualification of arbitrary
procedural macros, native compilers or shared Cargo target directories.
"""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def run(command, *, cwd, env):
    result = subprocess.run(command, cwd=cwd, env=env, text=True,
                            capture_output=True, timeout=90, check=False)
    if result.returncode:
        raise RuntimeError(f"{Path(command[0]).name} failed ({result.returncode}): {result.stderr}")
    return result.stdout.strip()


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    # Reproduce the old-mtime condition without changing compiler-cache files.
    os.utime(path, (946684800, 946684800))


def qualify(executable, toolchain, work_root):
    executable = executable.resolve(strict=True)
    cargo = shutil.which("cargo")
    if not cargo:
        raise ValueError("cargo is not installed on PATH")
    work_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="compiler-cache-", dir=work_root) as directory:
        root = Path(directory).resolve()
        config = root / "sccache.toml"
        config.write_text(f'[cache.disk]\ndir = {json.dumps(str(root / "cache"))}\nsize = 134217728\n')
        # Keep credentials, remote-cache settings and other wrappers out of this
        # local fixture. HOME is preserved only for the installed Rust toolchain.
        env = {key: os.environ[key] for key in ("PATH", "HOME", "TMPDIR", "RUSTUP_HOME", "CARGO_HOME") if key in os.environ}
        env.update(SCCACHE_CONF=str(config), SCCACHE_SERVER_UDS=str(root / "cache.sock"),
                   SCCACHE_IDLE_TIMEOUT="60", CARGO_INCREMENTAL="0", RUSTC_WRAPPER=str(executable))
        shared = root / "shared"
        write(shared / "Cargo.toml", '[package]\nname="cache-shared-probe"\nversion="0.1.0"\nedition="2021"\n[workspace]\n')
        write(shared / "src/lib.rs", 'pub fn value() -> &' + "'static str { include_str!(\"value.txt\") }\n")
        write(shared / "src/value.txt", "shared-one")
        observed = []
        for name, value in (("copy-a", "A"), ("copy-b", "B"), ("copy-c", "C")):
            checkout = root / name
            manifest = ('[package]\nname="cache-source-probe"\nversion="0.1.0"\nedition="2021"\n'
                        '[workspace]\n[dependencies]\ncache-shared-probe={path="../shared"}\n')
            write(checkout / "Cargo.toml", manifest)
            write(checkout / "src/lib.rs", f'pub fn value() -> &' + f"'static str {{ \"{value}\" }}\n")
            write(checkout / "src/main.rs", 'fn main() { println!("{}:{}", cache_source_probe::value(), cache_shared_probe::value()); }\n')
        started = False
        try:
            run([str(executable), "--start-server"], cwd=root, env=env)
            started = True
            for name, expected in (("copy-a", "A:shared-one"), ("copy-b", "B:shared-one"), ("copy-c", "C:shared-two")):
                if name == "copy-c":
                    write(shared / "src/value.txt", "shared-two")
                checkout = root / name
                output = run([cargo, f"+{toolchain}", "run", "--quiet", "--offline", "--target-dir", str(checkout / "target")], cwd=checkout, env=env)
                if output != expected:
                    raise AssertionError(f"{name}: expected {expected!r}, received {output!r}")
                observed.append({"copy": name, "output": output})
            stats = json.loads(run([str(executable), "--show-stats", "--stats-format", "json"], cwd=root, env=env))
            hits = stats["stats"]["cache_hits"]["counts"].get("Rust", 0)
            if hits < 1:
                raise AssertionError(f"No Rust compiler-cache hit was established: {stats}")
            return {"toolchain": toolchain, "sccache": run([str(executable), "--version"], cwd=root, env=env),
                    "outputs": observed, "rust_cache_hits": hits, "distinct_targets": True,
                    "old_source_timestamps": True, "changed_include_observed": True}
        finally:
            if started:
                run([str(executable), "--stop-server"], cwd=root, env=env)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sccache", type=Path, required=True)
    parser.add_argument("--toolchain", required=True)
    parser.add_argument("--work-root", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(qualify(args.sccache, args.toolchain, args.work_root), indent=2))


if __name__ == "__main__":
    main()

// Native signed transport and independent crypto; every seed here is public.
import Foundation
import Security
import CryptoKit

func check(_ condition: Bool, _ name: String) {
    if !condition { fputs("native signing case failed: \(name)\n", stderr); exit(1) }
}

func unhex(_ input: String) -> Data {
    var result = Data(); var index = input.startIndex
    while index < input.endIndex {
        let end = input.index(index, offsetBy: 2)
        result.append(UInt8(input[index..<end], radix: 16)!); index = end
    }
    return result
}

let domains: [String: String] = [
    "session_admission_lease": "PALE_SESSION_ADMISSION_LEASE_V1\0",
    "operation_handle": "PALE_OPERATION_HANDLE_V1\0",
    "operation_observation": "PALE_OPERATION_OBSERVATION_V1\0",
    "a2a_task_binding": "PALE_A2A_TASK_BINDING_V1\0",
    "a2a_task_observation": "PALE_A2A_TASK_OBSERVATION_V1\0",
    "capability_result": "PALE_CAPABILITY_RESULT_V1\0",
    "adapter_failure": "PALE_ADAPTER_FAILURE_V1\0",
    "control_state_checkpoint": "PALE_CONTROL_STATE_CHECKPOINT_V1\0",
    "principal_trust_checkpoint": "PALE_PRINCIPAL_TRUST_CHECKPOINT_V1\0"
]

@available(macOS 13.0, *)
final class SlowFixture: NSObject, NSXPCListenerDelegate, PaleKeychainSigning {
    let requirement: String
    let completed = DispatchSemaphore(value: 0)
    let lock = NSLock()
    var requests = 0
    init(requirement: String) { self.requirement = requirement }
    func listener(_ listener: NSXPCListener, shouldAcceptNewConnection connection: NSXPCConnection) -> Bool {
        connection.setCodeSigningRequirement(requirement)
        connection.exportedInterface = NSXPCInterface(with: PaleKeychainSigning.self)
        connection.exportedObject = self; connection.resume(); return true
    }
    func sign(_ keyAlias: String, recordKind: String, digest: Data,
              budgetMilliseconds: UInt32, withReply reply: @escaping (Data?, UInt32) -> Void) {
        lock.lock(); requests += 1; lock.unlock()
        DispatchQueue.global().asyncAfter(deadline: .now() + .milliseconds(150)) {
            reply(Data(repeating: 0, count: 64), 0); self.completed.signal()
        }
    }
}

@available(macOS 13.0, *)
func exercise(_ requirement: String) throws {
    check(pale_darwin_signing_is_fixture_v1() == 1, "public fixture archive required before dispatch")
    check(MemoryLayout<PaleSigningRequestV1>.size == 4100 && MemoryLayout<PaleSigningResponseV1>.size == 68, "fixed C ABI")
    let publicBytes = unhex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
    let publicKey = try Curve25519.Signing.PublicKey(rawRepresentation: publicBytes)
    let binding = try SigningHelperKeyBinding(service: "qualification-only", account: "public-rfc8032-vector-1",
        accessGroup: nil, publicKey: publicBytes, allowedKinds: Set(domains.keys))
    let wrong = try SigningHelperKeyBinding(service: "qualification-only", account: "public-rfc8032-vector-1",
        accessGroup: nil, publicKey: Data(repeating: 0, count: 32), allowedKinds: ["operation_handle"])
    let listener = NSXPCListener.anonymous()
    let helper = try SigningHelperListener(listener: listener, clientRequirement: requirement, clientUID: geteuid(),
        maximumConnections: 8, slots: ["qualified": binding, "wrong-public-key": wrong])
    helper.resume(); defer { helper.invalidate() }
    let client = try SigningHelperClient(connection: NSXPCConnection(listenerEndpoint: listener.endpoint), helperRequirement: requirement)
    defer { client.invalidate() }
    let digest = Data(repeating: 0xaa, count: 32)
    var names = ["fixed C ABI"]
    for kind in domains.keys.sorted() {
        let result = client.sign(keyAlias: "qualified", recordKind: kind, digest: digest, budgetMilliseconds: 2000)
        guard case let .success(signature) = result else { check(false, "signed XPC \(kind): \(result)"); return }
        var message = Data(domains[kind]!.utf8); message.append(digest)
        check(publicKey.isValidSignature(signature, for: message), "independent CryptoKit \(kind)")
        message[message.count - 1] ^= 1
        check(!publicKey.isValidSignature(signature, for: message), "altered digest \(kind)")
        names.append("signed XPC and independent crypto: \(kind)")
    }
    if case .failure(.refused(100)) = client.sign(keyAlias: "absent", recordKind: "operation_handle", digest: digest, budgetMilliseconds: 2000) {}
    else { check(false, "unknown alias"); return }; names.append("unknown alias refused")
    if case .failure(.refused(102)) = client.sign(keyAlias: "qualified", recordKind: "arbitrary_domain", digest: digest, budgetMilliseconds: 2000) {}
    else { check(false, "unlisted kind"); return }; names.append("unlisted kind refused")
    if case .failure(.refused(5)) = client.sign(keyAlias: "wrong-public-key", recordKind: "operation_handle", digest: digest, budgetMilliseconds: 2000) {}
    else { check(false, "unlike public key"); return }; names.append("unlike public key refused")
    if case .failure(.invalidInput) = client.sign(keyAlias: "qualified", recordKind: "operation_handle", digest: Data(), budgetMilliseconds: 2000) {}
    else { check(false, "short digest"); return }; names.append("short digest refused locally")
    let denied = try SigningHelperClient(connection: NSXPCConnection(listenerEndpoint: listener.endpoint),
        helperRequirement: "cdhash H\"0000000000000000000000000000000000000000\"")
    defer { denied.invalidate() }
    if case .failure(.unavailable) = denied.sign(keyAlias: "qualified", recordKind: "operation_handle", digest: digest, budgetMilliseconds: 2000) {}
    else { check(false, "wrong helper code requirement"); return }; names.append("wrong helper requirement refused")
    do {
        _ = try SigningHelperListener(listener: NSXPCListener.anonymous(), clientRequirement: "invalid requirement syntax !",
            clientUID: geteuid(), maximumConnections: 1, slots: ["qualified": binding])
        check(false, "malformed code requirement")
    } catch { names.append("malformed code requirement refused") }
    for (name, clientRule, uid) in [
        ("wrong caller requirement refused", "cdhash H\"0000000000000000000000000000000000000000\"", geteuid()),
        ("wrong caller UID refused", requirement, geteuid() ^ 1)
    ] {
        let nativeListener = NSXPCListener.anonymous()
        let denyingHelper = try SigningHelperListener(listener: nativeListener, clientRequirement: clientRule,
            clientUID: uid, maximumConnections: 1, slots: ["qualified": binding])
        denyingHelper.resume(); defer { denyingHelper.invalidate() }
        let nativeClient = try SigningHelperClient(connection: NSXPCConnection(listenerEndpoint: nativeListener.endpoint),
            helperRequirement: requirement)
        defer { nativeClient.invalidate() }
        if case .failure(.unavailable) = nativeClient.sign(keyAlias: "qualified", recordKind: "operation_handle",
            digest: digest, budgetMilliseconds: 2000) {} else { check(false, name); return }
        names.append(name)
    }
    let slowListener = NSXPCListener.anonymous()
    let slow = SlowFixture(requirement: requirement); slowListener.delegate = slow
    slowListener.resume(); defer { slowListener.invalidate() }
    let slowClient = try SigningHelperClient(connection: NSXPCConnection(listenerEndpoint: slowListener.endpoint),
        helperRequirement: requirement)
    defer { slowClient.invalidate() }
    let timeout = slowClient.sign(keyAlias: "fixture", recordKind: "operation_handle", digest: digest, budgetMilliseconds: 20)
    if case .failure(.deadline) = timeout {} else { check(false, "bounded client wait"); return }
    check(slow.completed.wait(timeout: .now() + .seconds(2)) == .success, "actual late native reply")
    slow.lock.lock(); let requestCount = slow.requests; slow.lock.unlock()
    check(requestCount == 1, "no automatic retry")
    names.append("deadline retained after actual late reply; no retry")
    check(names.count == 19, "complete native case set")
    let result: [String: Any] = ["native_cases_passed": names.count, "failures": 0,
        "keychain_access": "none: compiled public RFC fixture only", "cases": names,
        "scope": "Actual ad-hoc-signed same-process anonymous XPC; not separate-process isolation or production identity"]
    let output = try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys])
    print(String(data: output, encoding: .utf8)!)
}

check(CommandLine.arguments.count == 2, "explicit fixture code requirement")
let complete = DispatchSemaphore(value: 0)
DispatchQueue.global().async {
    do { try exercise(CommandLine.arguments[1]); complete.signal() }
    catch { fputs("native signing fixture failed: \(error)\n", stderr); exit(1) }
}
let deadline = Date().addingTimeInterval(20)
while complete.wait(timeout: .now()) == .timedOut {
    check(Date() < deadline, "bounded native fixture")
    RunLoop.current.run(until: Date().addingTimeInterval(0.005))
}

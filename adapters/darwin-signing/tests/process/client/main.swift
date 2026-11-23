// The process client links only the shared protocol and client, never the core.
import Foundation
import CryptoKit

func check(_ condition: Bool, _ name: String) {
    if !condition { fputs("process signing case failed: \(name)\n", stderr); exit(1) }
}

check(CommandLine.arguments.count == 3, "explicit service and helper requirement")
let complete = DispatchSemaphore(value: 0)
DispatchQueue.global().async {
    do {
        let connection = NSXPCConnection(serviceName: CommandLine.arguments[1])
        let client = try SigningHelperClient(connection: connection, helperRequirement: CommandLine.arguments[2])
        defer { client.invalidate() }
        let digest = Data(repeating: 0xaa, count: 32)
        let result = client.sign(keyAlias: "qualified", recordKind: "operation_handle", digest: digest, budgetMilliseconds: 5000)
        guard case let .success(signature) = result else { check(false, "native signing: \(result)"); return }
        let publicBytes: [UInt8] = [0xd7,0x5a,0x98,0x01,0x82,0xb1,0x0a,0xb7,0xd5,0x4b,0xfe,0xd3,0xc9,0x64,0x07,0x3a,
            0x0e,0xe1,0x72,0xf3,0xda,0xa6,0x23,0x25,0xaf,0x02,0x1a,0x68,0xf7,0x07,0x51,0x1a]
        let key = try Curve25519.Signing.PublicKey(rawRepresentation: Data(publicBytes))
        var message = Data("PALE_OPERATION_HANDLE_V1\0".utf8); message.append(digest)
        check(key.isValidSignature(signature, for: message), "independent signature verification")
        let peer = connection.processIdentifier
        check(peer > 0 && peer != getpid(), "separate kernel-observed helper PID")
        let resultObject: [String: Any] = ["caller_pid": getpid(), "helper_pid": peer,
            "signature_verified": true, "client_has_signing_core": false,
            "keychain_access": "none: compiled public RFC fixture only"]
        let bytes = try JSONSerialization.data(withJSONObject: resultObject, options: [.sortedKeys])
        print(String(data: bytes, encoding: .utf8)!); complete.signal()
    } catch { fputs("process signing fixture failed: \(error)\n", stderr); exit(1) }
}
let deadline = Date().addingTimeInterval(10)
while complete.wait(timeout: .now()) == .timedOut {
    check(Date() < deadline, "bounded process client")
    RunLoop.current.run(until: Date().addingTimeInterval(0.005))
}

// Native XPC policy and helper dispatch. Rust owns cryptographic record domains.
import Foundation

public enum SigningHelperConfigurationError: Error {
    case invalidPolicy
}

/// A helper-owned slot. Callers supply its alias; they cannot replace its native
/// Keychain coordinates, expected public key or allowed record kinds.
public struct SigningHelperKeyBinding {
    let service: String
    let account: String
    let accessGroup: String?
    let publicKey: Data
    let allowedKinds: Set<String>

    public init(service: String, account: String, accessGroup: String?,
                publicKey: Data, allowedKinds: Set<String>) throws {
        guard validSigningCoordinate(service), validSigningCoordinate(account),
              accessGroup.map({ validSigningCoordinate($0) }) ?? true,
              publicKey.count == 32, !allowedKinds.isEmpty, allowedKinds.count <= 9,
              allowedKinds.allSatisfy({ validSigningCoordinate($0, maximumBytes: 64) })
        else { throw SigningHelperConfigurationError.invalidPolicy }
        self.service = service; self.account = account; self.accessGroup = accessGroup
        self.publicKey = publicKey; self.allowedKinds = allowedKinds
    }
}

@available(macOS 13.0, *)
public final class SigningHelperListener: NSObject, NSXPCListenerDelegate {
    private let listener: NSXPCListener
    private let clientRequirement: String
    private let clientUID: uid_t
    private let maximumConnections: Int
    private let slots: [String: SigningHelperKeyBinding]
    private let lock = NSLock()
    private var connections = Set<ObjectIdentifier>()

    /// The composition owner supplies a listener and actual protected policy.
    /// Construction starts no service and chooses no client identity or item.
    public init(listener: NSXPCListener, clientRequirement: String, clientUID: uid_t,
                maximumConnections: Int, slots: [String: SigningHelperKeyBinding]) throws {
        guard validSigningRequirement(clientRequirement), (1...64).contains(maximumConnections),
              !slots.isEmpty, slots.count <= 64,
              slots.keys.allSatisfy({ validSigningCoordinate($0, maximumBytes: 128) })
        else { throw SigningHelperConfigurationError.invalidPolicy }
        self.listener = listener; self.clientRequirement = clientRequirement
        self.clientUID = clientUID; self.maximumConnections = maximumConnections; self.slots = slots
        super.init()
        listener.delegate = self
    }

    public func resume() { listener.resume() }
    public func invalidate() { listener.invalidate() }

    public func listener(_ listener: NSXPCListener, shouldAcceptNewConnection connection: NSXPCConnection) -> Bool {
        guard connection.effectiveUserIdentifier == clientUID else { return false }
        let identity = ObjectIdentifier(connection)
        lock.lock()
        guard connections.count < maximumConnections else { lock.unlock(); return false }
        connections.insert(identity)
        lock.unlock()
        connection.setCodeSigningRequirement(clientRequirement)
        connection.exportedInterface = NSXPCInterface(with: PaleKeychainSigning.self)
        connection.exportedObject = SigningHelperSession(slots: slots)
        connection.invalidationHandler = { [weak self] in
            guard let self = self else { return }
            self.lock.lock(); self.connections.remove(identity); self.lock.unlock()
        }
        connection.resume()
        return true
    }
}

private final class SigningHelperSession: NSObject, PaleKeychainSigning {
    private let slots: [String: SigningHelperKeyBinding]
    init(slots: [String: SigningHelperKeyBinding]) { self.slots = slots }

    func sign(_ keyAlias: String, recordKind: String, digest: Data,
              budgetMilliseconds: UInt32, withReply reply: @escaping (Data?, UInt32) -> Void) {
        guard validSigningCoordinate(keyAlias, maximumBytes: 128),
              validSigningCoordinate(recordKind, maximumBytes: 64), digest.count == 32,
              (1...30_000).contains(budgetMilliseconds) else { reply(nil, 101); return }
        guard let slot = slots[keyAlias] else { reply(nil, 100); return }
        guard slot.allowedKinds.contains(recordKind) else { reply(nil, 102); return }
        let publicKey = slot.publicKey.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
        var object: [String: Any] = [
            "schema_version": 1, "kind": recordKind,
            "digest": digest.map { String(format: "%02x", $0) }.joined(),
            "service": slot.service, "account": slot.account,
            "public_key": publicKey, "budget_ms": budgetMilliseconds
        ]
        if let group = slot.accessGroup { object["access_group"] = group }
        guard let bytes = try? JSONSerialization.data(withJSONObject: object), bytes.count <= 4096
        else { reply(nil, 103); return }
        var frame = PaleSigningRequestV1()
        frame.length = UInt32(bytes.count)
        withUnsafeMutableBytes(of: &frame.bytes) { $0.copyBytes(from: bytes) }
        let response = pale_darwin_signing_execute_v1(frame)
        guard response.status == 0 else { reply(nil, response.status); return }
        reply(withUnsafeBytes(of: response.signature) { Data($0) }, 0)
    }
}

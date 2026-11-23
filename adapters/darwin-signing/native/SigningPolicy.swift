// Capture through the filesystem owner, then project the closed Rust policy.
import Foundation
import CryptoKit

public enum SigningPolicyCaptureError: Error {
    case invalidPath
    case captureRefused(UInt32)
    case invalidProjection
}

private struct PolicyWire: Decodable {
    let schema_version: UInt8
    let client_requirement: String
    let client_uid: UInt32
    let maximum_connections: Int
    let slots: [SlotWire]
}

private struct SlotWire: Decodable {
    let alias: String
    let service: String
    let account: String
    let access_group: String?
    let public_key: String
    let allowed_kinds: [String]
}

/// A captured configuration view, not a grant, live lease or activated service.
@available(macOS 13.0, *)
public struct CapturedSigningHelperPolicy {
    public let sourceSHA256: String
    private let clientRequirement: String
    private let clientUID: uid_t
    private let maximumConnections: Int
    private let slots: [String: SigningHelperKeyBinding]

    /// Load only the explicitly selected native file. The Rust filesystem owner
    /// enforces no-follow traversal, private POSIX protection and 16 KiB bound;
    /// Rust also validates the closed shape before bytes reach this projection.
    /// No search, permission repair, Keychain lookup or listener start occurs.
    public static func load(explicitPath: String) throws -> CapturedSigningHelperPolicy {
        let path = Data(explicitPath.utf8)
        guard !path.isEmpty, path.count <= 4096 else { throw SigningPolicyCaptureError.invalidPath }
        var request = PaleSigningRequestV1()
        request.length = UInt32(path.count)
        withUnsafeMutableBytes(of: &request.bytes) { $0.copyBytes(from: path) }
        let captured = pale_darwin_signing_capture_policy_v1(request)
        guard captured.status == 0 else { throw SigningPolicyCaptureError.captureRefused(captured.status) }
        guard captured.length <= 16384 else { throw SigningPolicyCaptureError.invalidProjection }
        var bytes = Data()
        withUnsafeBytes(of: captured.block0) { bytes.append(contentsOf: $0) }
        withUnsafeBytes(of: captured.block1) { bytes.append(contentsOf: $0) }
        withUnsafeBytes(of: captured.block2) { bytes.append(contentsOf: $0) }
        withUnsafeBytes(of: captured.block3) { bytes.append(contentsOf: $0) }
        bytes = bytes.prefix(Int(captured.length))
        guard let policy = try? JSONDecoder().decode(PolicyWire.self, from: bytes), policy.schema_version == 1,
              validSigningRequirement(policy.client_requirement) else { throw SigningPolicyCaptureError.invalidProjection }
        var slots: [String: SigningHelperKeyBinding] = [:]
        for slot in policy.slots {
            var base64 = slot.public_key.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/")
            base64 += String(repeating: "=", count: (4 - base64.utf8.count % 4) % 4)
            guard let key = Data(base64Encoded: base64), slots[slot.alias] == nil
            else { throw SigningPolicyCaptureError.invalidProjection }
            slots[slot.alias] = try SigningHelperKeyBinding(service: slot.service, account: slot.account,
                accessGroup: slot.access_group, publicKey: key, allowedKinds: Set(slot.allowed_kinds))
        }
        return CapturedSigningHelperPolicy(sourceSHA256: SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined(),
            clientRequirement: policy.client_requirement, clientUID: policy.client_uid,
            maximumConnections: policy.maximum_connections, slots: slots)
    }

    public func makeListener(_ listener: NSXPCListener) throws -> SigningHelperListener {
        try SigningHelperListener(listener: listener, clientRequirement: clientRequirement,
            clientUID: clientUID, maximumConnections: maximumConnections, slots: slots)
    }
}

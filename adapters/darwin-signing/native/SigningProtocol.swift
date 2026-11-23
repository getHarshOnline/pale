// Shared native protocol and validation. No signing core or Keychain lookup.
import Foundation
import Security

@objc public protocol PaleKeychainSigning {
    func sign(_ keyAlias: String, recordKind: String, digest: Data,
              budgetMilliseconds: UInt32, withReply reply: @escaping (Data?, UInt32) -> Void)
}

func validSigningRequirement(_ value: String) -> Bool {
    guard !value.isEmpty, value.utf8.count <= 4096 else { return false }
    var requirement: SecRequirement?
    return SecRequirementCreateWithString(value as CFString, [], &requirement) == errSecSuccess
}

func validSigningCoordinate(_ value: String, maximumBytes: Int = 256) -> Bool {
    !value.isEmpty && value.utf8.count <= maximumBytes
        && value.unicodeScalars.allSatisfy { !CharacterSet.controlCharacters.contains($0) }
}

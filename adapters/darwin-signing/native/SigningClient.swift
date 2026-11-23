// Bounded native client; it carries no Keychain coordinates or private keys.
import Foundation

public enum SigningHelperClientError: Error {
    case invalidInput
    case unavailable
    case deadline
    case refused(UInt32)
    case invalidResponse
}

private final class SigningReply {
    let lock = NSLock()
    let ready = DispatchSemaphore(value: 0)
    let deadline: DispatchTime
    var result: Result<Data, SigningHelperClientError>?

    init(deadline: DispatchTime) { self.deadline = deadline }

    func finish(_ value: Result<Data, SigningHelperClientError>) {
        lock.lock(); defer { lock.unlock() }
        guard result == nil else { return }
        result = DispatchTime.now().uptimeNanoseconds >= deadline.uptimeNanoseconds ? .failure(.deadline) : value
        ready.signal()
    }

    func wait() -> Result<Data, SigningHelperClientError> {
        let status = ready.wait(timeout: deadline)
        lock.lock(); defer { lock.unlock() }
        if status == .timedOut { result = .failure(.deadline) }
        return result ?? .failure(.unavailable)
    }
}

@available(macOS 13.0, *)
public final class SigningHelperClient {
    private let connection: NSXPCConnection

    /// Caller selects the native connection and admitted helper requirement.
    /// A path, service name or same UID is not a code-signing requirement.
    public init(connection: NSXPCConnection, helperRequirement: String) throws {
        guard validSigningRequirement(helperRequirement) else { throw SigningHelperClientError.invalidInput }
        self.connection = connection
        connection.setCodeSigningRequirement(helperRequirement)
        connection.remoteObjectInterface = NSXPCInterface(with: PaleKeychainSigning.self)
        connection.resume()
    }

    public func invalidate() { connection.invalidate() }

    /// One request, one bounded wait, no reconnect or automatic retry. A late
    /// reply cannot replace the retained deadline result. This synchronous call
    /// belongs outside the host's main thread or async executor.
    public func sign(keyAlias: String, recordKind: String, digest: Data,
                     budgetMilliseconds: UInt32) -> Result<Data, SigningHelperClientError> {
        guard validSigningCoordinate(keyAlias, maximumBytes: 128),
              validSigningCoordinate(recordKind, maximumBytes: 64), digest.count == 32,
              (1...30_000).contains(budgetMilliseconds) else { return .failure(.invalidInput) }
        let reply = SigningReply(deadline: .now() + .milliseconds(Int(budgetMilliseconds)))
        guard let remote = connection.remoteObjectProxyWithErrorHandler({ _ in
            reply.finish(.failure(.unavailable))
        }) as? PaleKeychainSigning else { return .failure(.unavailable) }
        remote.sign(keyAlias, recordKind: recordKind, digest: digest,
                    budgetMilliseconds: budgetMilliseconds) { bytes, status in
            if status != 0 { reply.finish(.failure(.refused(status))); return }
            guard let bytes = bytes, bytes.count == 64 else { reply.finish(.failure(.invalidResponse)); return }
            reply.finish(.success(bytes))
        }
        return reply.wait()
    }
}

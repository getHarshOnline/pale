// Application-scoped XPC service. No fallback policy, installation or key setup.
import Foundation

#if PALE_SIGNING_QUALIFICATION
// Owned test service only; no fixture may remain alive after its bounded run.
guard pale_darwin_signing_is_fixture_v1() == 1 else { exit(78) }
DispatchQueue.global().asyncAfter(deadline: .now() + .seconds(20)) { _exit(0) }
#else
guard pale_darwin_signing_is_fixture_v1() == 0 else { exit(78) }
#endif

guard CommandLine.arguments.count == 1,
      let path = Bundle.main.object(forInfoDictionaryKey: "PaleSigningPolicyPath") as? String
else { exit(78) }
do {
    let policy = try CapturedSigningHelperPolicy.load(explicitPath: path)
    let helper = try policy.makeListener(NSXPCListener.service())
    // Apple's service listener hands over to XPC and does not return.
    withExtendedLifetime(helper) { helper.resume() }
} catch {
    // Native owner can observe the process exit; no path or policy is logged.
    exit(78)
}

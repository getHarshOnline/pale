# Keep the guarantees the change must respect

After understanding the change, identify the guarantees every acceptable solution must keep. Correctness, security, accessibility and explicitly requested behaviour take priority over fewer lines, lower testing cost or a simpler-looking implementation.

For the affected behaviour, find where untrusted input enters, where identity and permission are checked, who owns protected data and state, how failures are contained, how people access the result, what outside systems may change, and how recovery works. In the working analysis, name each guarantee and what happens if it fails. Carry any threatened guarantee into the implementation and its checks.

Keep these distinctions and protections:

- Validate and interpret input wherever it enters from an untrusted source.
- Check identity, permission and purpose separately, and grant only the access needed.
- Keep secrets and sensitive data within their permitted destinations, including logs, artifacts and errors.
- After partial failure, retain consistent state or a recovery record that identifies what happened and who is responsible.
- Keep affected tasks accessible, including their alternative interaction paths.
- Preserve explicitly required behaviour and the interfaces existing users rely on.
- When software interprets physical measurements or controls physical effects, check calibration and the tolerances supported by real-world measurements.

Before a hard-to-reverse, destructive, public, financial, production or safety-related action, establish its exact target, existing permission, maximum scope, and how to recover or compensate if it fails. If state may have changed but the result cannot be read back, keep that uncertainty visible and investigate before retrying or claiming success.

Check ownership by following the actual relationship: who defines the fact, who may change it, what passes to the other system and who can check the result. A project can refer to another domain, provider or product while that owner keeps its data and decisions. Removing the owner's name from code does not change that relationship.

In the existing implementation plan or reply, explain any guarantee at risk, the protection it needs, shortcuts that would break it, recovery still required and decisions still with an owner. Then check the consuming system's actual enforcement; this guidance supplies the reasoning, while that system must apply it.

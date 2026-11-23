# Check TypeScript types against runtime data

Apply this guidance when TypeScript is selected. JavaScript alone, including a React or Vue project, does not establish that TypeScript is in use. Read the actual compiler options and identify where runtime data is validated before relying on a static type.

Give public interfaces precise types. Use discriminated unions for states whose cases must be handled explicitly, and narrow `unknown` before trusting it. An `any`, assertion, non-null claim or unchecked generic needs a checked condition that supplies the guarantee the type system no longer enforces.

Prefer immutable data flow where it makes ownership clearer. Give each promise an owner: await it or deliberately return it, with failures handled and parallel work chosen explicitly. Validate parsed values, environment variables, network responses, stored data and messages when they enter the program.

Inspect dynamic execution, object merging, URLs, HTML, queries, processes, paths and client bundles. Keep secrets and server-only values out of emitted JavaScript and public source maps.

Run the repository's configured type check and exercise behaviour wherever a type check is bypassed or external data enters. In the reply, say which runtime checks support the type claim and what remains unchecked.

## Validate a response variant

Treat a response body as `unknown`. Check its discriminant and fields before narrowing it to a success or error variant. Exercise a response with the right tag but a missing payload, so the caller cannot mistake unchecked data for a valid result.

## Before handing off

- [ ] Is external data narrowed before it becomes a domain value?
- [ ] Are state, promise ownership and failures explicit?
- [ ] Were the real caller and invalid input path exercised?

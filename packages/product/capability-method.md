# Define what a product must enable

Turn an accepted need into a clear statement of what the product must let someone do. Settle that meaning before dividing implementation among interfaces and services.

## Start with the need and the existing product

Read the validated problem, desired outcome, affected people or systems, current behavior, applicable constraints and evidence of the need. A roadmap label or proposed screen alone leaves the required ability unclear.

## Describe the ability before choosing its implementation

State the ability in terms a user or another system can observe. Keep the choice of interface, service or vendor open unless it has already been settled. Explain what must remain true in every implementation, what information passes between parts, who may read or change it, how failures behave and how success can be measured.

Specify an interface contract where independent parts must coordinate. Separate settled decisions from questions that must be answered before work can be divided. Map the ability onto the product's existing parts and identify which current capabilities it uses. If it duplicates an existing ability under a new feature name, resolve that overlap before planning delivery.

## Return a proposal the owner can use

In the capability proposal, include who uses it, their outcomes, the conditions that must always hold, interfaces between independent parts, success measures, dependencies, exclusions and open decisions. Finish when planners can divide the work without inventing what the product is meant to do. The product's owner still decides its implementation and maintains its own specifications and records.

## Worked example

A requester needs to find the outcome of work submitted earlier. Describe the ability to retrieve the current status and next available action for their own request. An old link must still show whether the request is pending, complete or no longer available. Test those outcomes and a link to a request the caller cannot see. Choose the interface once the required behavior is clear.

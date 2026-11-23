# Review a machine-learning change

Use this perspective when a change affects learned behaviour or the evidence used to accept it. Start with the decision the model supports. Trace the claimed result backward through evaluation, training, feature construction and data knowable at prediction time. Treat a favourable metric as unsupported if it uses future information, hides a consequential population in an aggregate or comes from a comparison chosen after seeing its outcome.

Check that the experiment and the running system use compatible inputs and transformations. The evidence should identify the model and transformation state well enough to reproduce the claim and explain retries, fallback, rollback and delayed labels. Resource use, latency, calibration and business constraints are requirements alongside predictive quality.

Follow sensitive information and changing artifacts through their full lifecycle. In the review, identify the earliest unsupported step, its downstream consequence and evidence needed to settle it. Add language, data-store, security or infrastructure review only when the agreed scope calls for it. Missing environment evidence limits the conclusion; it does not establish a way to run the system.

# Decide whether a product needs machine learning

Use learning when variation in data can improve an observable decision more reliably than rules, search, a workflow or an ordinary software change. Judge the value of a maintainable model against its cost and risk.

Identify the user or operational decision, prediction or generation target, time of decision, consequences of errors, current behaviour, available labels or feedback, privacy and use rights, and who decides acceptance. Test simpler alternatives first. A heuristic or manual workflow may work better when data is sparse, conditions change faster than learning, explanations are mandatory or different errors have very different costs.

Check whether training examples represent the deployment population, whether the target can be observed without leaking the answer, whether features are available at decision time and whether feedback can distinguish the model's value from surrounding product changes. Define a simple baseline and an end-to-end shadow run or limited trial before adding model complexity.

Adoption needs an interface from which the model can be removed, measured improvement over the baseline, results for consequential subgroups, acceptable latency and cost, safe fallback, monitoring tied to the intended outcome, and an owner-controlled path for rollout and rollback. Good aggregate accuracy cannot compensate for failure on a small consequential class.

In the recommendation, choose adopt, investigate, redesign or decline. Explain the decision and baseline, data readiness, evaluation requirements, where the model fits, operational burden, risk, fallback and evidence that could change the recommendation. Training, promotion, serving and data acquisition require their own execution steps and permissions.

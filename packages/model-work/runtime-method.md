# Compare a model's full runtime on its intended hardware

## Fix the workload and system being compared

State the decision, current and proposed systems, claim, acceptable margin, hard limits and minimum requirements under failure. Identify the checkpoint and digest, input processing and output requirements, storage and execution precision, runtime and compiler builds, where operations actually run, target hardware, memory limits and sharing. Include queueing, batching or streaming, context and input size, concurrency and reduced-capability paths. Use the same permitted cases and measurement start and end points. Check cold and warm conditions separately. If hardware, runtime or workload differs, identify the result as a proxy where appropriate. File size or another model's timing cannot establish peak runtime memory or end-to-end latency.

## Check artifact and backend compatibility before comparing runs

Fix the original checkpoint, configuration, tokenizer or processor, required special tokens, template and generation settings. Identify converted artifacts by conversion inputs, tool version, parameters and digests. Distinguish source precision, stored-weight quantisation, runtime precision and cache quantisation. Successful conversion establishes that an artifact was produced. Loading, execution on the chosen hardware and each claimed capability need their own observations.

| Selected route | Check before claiming support |
| --- | --- |
| MLX LM | Match the pinned configuration to the installed runtime's architecture implementation. Bind the converted weights, conversion or quantization settings and tokenizer. Observe loading and the required generation path, including streaming and actual device placement where claimed. [MLX LM](https://github.com/ml-explore/mlx-lm) |
| llama.cpp and GGUF | Check that the exact converter and runtime both support the architecture. Inspect the converted metadata, tokenizer and tensor mapping and bind the quantization recipe. Run the installed backend against that exact artifact. A serving wrapper needs a separate check of the selected underlying artifact, request formatting, defaults and returned capabilities. For Ollama, bind the effective Modelfile and invocation as well. [llama.cpp model integration guide](https://github.com/ggml-org/llama.cpp/blob/master/docs/development/HOWTO-add-model.md), [Ollama Modelfile](https://docs.ollama.com/modelfile) |
| Transformers | Resolve the exact model configuration and AutoClass mapping, the selected weight file or shard set and its tensor contract, and the tokenizer or processor files. Read back the loaded class and test the claimed operation. Loading repository-supplied executable code needs separate admission of the exact revision. [Transformers model loading](https://huggingface.co/docs/transformers/en/models) |
| Python extension on the runtime path | Bind the wheel, Python implementation and version, platform and architecture, ABI and linked native libraries. Install and import the selected wheel in a clean supported environment and exercise the required entry point. A stable ABI tag leaves platform and linked-library checks to the selected target. [Maturin bindings](https://www.maturin.rs/bindings.html) |

For each capability, inspect the path actually used and a representative request through the consumer's interface. Test a known-good artifact, a missing or mismatched required asset and an unsupported capability. If a wrapper can silently switch artifact, format, backend or precision, test substitution or fallback too. In the review, classify each result as supported, degraded, refused or unobserved. Use the project's own workload and acceptance requirements to decide whether the checked route is sufficient.

## Distinguish test conditions from how results are measured

Fix source cases, references, processing, measurement method and permitted operations. Specify where each condition enters the system, how its level is measured and the resource budget that applies. A perturbation can change what reaches the measurement without changing the measurement itself. Version a preprocessing step, decision threshold or record channel separately when it changes what the observer can see, and preserve evidence of that change.

To compare conditions, keep the route, measurement and reference fixed and vary the condition on paired cases. Record its realised level and the actual inputs consumed. To compare measurement methods, hold input and condition fixed. When both can change, test them separately and together if their interaction matters to the decision.

Report quality and resource coverage for each tested condition and relevant subgroup. State the aggregation unit and denominator, including unavailable and partial results. Exercise an unchanged baseline, a condition whose measured level contradicts its label, and substitution of an input or measurement method. Performance under a condition supports that particular robustness finding. Claims about another observer, field population or causal mechanism require their own evidence.

## Measure quality and resources together

Define when input is ready, the first useful output arrives and work completes. Say whether timing includes collection, lookahead, queueing, startup, decoding, transfers and fallback. Separate end-to-end latency, stage timing and sustained throughput. A streaming system can keep up overall while missing its interaction deadline. Repeat runs and report sample counts, the tail statistic and how it was estimated. Include timeouts, cancellations and partial outputs so failed calls remain in the denominator.

Measure peak memory for a named process, device or whole system. Account for resident weights, activations, runtime state, caches, buffers, copies and other workloads as relevant. Record where operations actually run and how much time, capacity and resources fallback consumes. Weight compression or sparse execution may leave other state resident. Measure energy or thermal limits on the target under the stated sustained load when they matter. Budget learning on the target separately, including its data, optimiser and gradient state, and any simultaneous serving obligation.

## Judge the same observed workload

Pair resource measurements with correctness, output validity and quality for critical subgroups on the same work. Test normal use, saturation, a missing or unsupported path, cancellation or timeout, and recovery to the accepted baseline. Combine stresses when their risks can occur together. Distinguish simulation, isolated-component checks and end-to-end runs on the target.

A missing observation for a mandatory requirement leaves it unresolved. A passing median or average cannot compensate for a failed tail or critical subgroup. In the comparison, say which claims pass, fail or remain unmeasured, with the evidence that could change each finding. The owning system and owner control execution, data rights, acceptable thresholds, promotion and rollback.

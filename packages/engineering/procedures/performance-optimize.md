# Improve a measured bottleneck

Start with one measured bottleneck, a representative workload, the behaviour that must stay correct and the project's measurement command. Use the benchmark and latency methods to establish comparable conditions and account for costs elsewhere in the system.

Within the agreed work, change one cause at a time and compare the resulting distributions, correctness and resource use. In the reply, give the original and changed measurements, any work moved elsewhere, checks for regressions and the decision to keep, revert or investigate further. Without a measured result after the change, describe it as a hypothesis and proposed improvement.

## Before handing off

- [ ] Was the original bottleneck measured under fixed conditions?
- [ ] Does the comparison isolate a relevant cause and check correctness as well as speed?
- [ ] Are shifted costs, uncertainty and regressions visible?

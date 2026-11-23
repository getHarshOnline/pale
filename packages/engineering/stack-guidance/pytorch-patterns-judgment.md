# Check PyTorch training evaluation and recovery

Use these read-only criteria to review reliable, efficient and reproducible PyTorch systems. Keep the run's seeds, configuration, environment, code and data versions with its results so another run can reproduce and compare them.

## Follow tensors and gradients through the model

Keep modules composable, tensor shapes understandable and model definition separate from training policy. A forward pass should not hide training effects. Establish the intended device, dtype, layout and shape for parameters, inputs, masks and newly created tensors, and investigate hidden transfers between host and device.

Make optimizer and scheduler order, gradient handling, precision and held-out evaluation consistent with the training loop. Match training or evaluation mode and gradient context to the phase. Check that in-place changes, early scalar extraction and detached tensors preserve the gradients the model needs.

Balance batching, workers, transforms and augmentation for throughput while preserving determinism where the comparison requires it.

## Resume and compare actual runs

A resumable checkpoint needs the model, optimizer, scheduler, precision scaler, random state and progress. Distinguish the latest checkpoint from the one selected by its metric.

Compare loss, quality, throughput and resource use against a simpler baseline. Keep mixed precision, gradient checkpointing or compilation when measured benefit outweighs numerical, memory, startup and debugging costs.

For example, run the same evaluation inputs under `model.eval()` with gradients disabled. Restore model and optimizer state before comparing a resumed training step. Check the data split, seed and tensor devices as well as the checkpoint.

## Before handing off

- [ ] Are tensor shapes, devices and training or evaluation mode correct?
- [ ] Can the data split and checkpointed run be reproduced?
- [ ] Was training or inference behaviour actually compared?

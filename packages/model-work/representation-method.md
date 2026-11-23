# Compare learned representations and sparse dictionaries

Use this guide to compare internal representations, reconstruction or feature differences. A representation score alone cannot establish task quality, clinical usefulness or the cause of a training change. The study owns its hypotheses, exact models, experimental definitions, data permissions, thresholds and conclusions.

Read `pale.evaluation:numerical-conformance` for checking the mathematical implementation and `pale.evaluation:candidate-comparison` for comparable alternatives, protected cases and intervention claims. During direct use, reach their source pages through the corpus guide; the operation combines them in a Pale profile.

## Establish which observations are paired

Identify the checkpoint, architecture, tokenizer or processor, template, activation hook and its position relative to the model block. Distinguish embedding output, zero-based block index and returned hidden-state index. Inspect loaded classes and activation shapes. A same-sized checkpoint or model-card label cannot establish shared training history or equivalent hooks.

Match each row by source case, prompt and the study's accepted token or sample alignment. State masks, padding, special and template tokens, truncation and excluded modalities. Equal row counts do not establish pairing. Different tokenizers need an explicitly justified alignment; joining their token indices directly may pair different content. Report the subgroups whose differences matter.

Fit learned preprocessing, whitening, normalisation and selection only on the training or calibration split permitted for that purpose. Distinguish centring required by the metric from fitted preprocessing. Preserve the transforms and their input history. Choosing a layer after inspecting evaluation results uses those results for selection; account for that in the later comparison.

## Check what similarity preserves

For paired matrices X and Y, centre their columns to obtain Xc and Yc. The usual linear centred kernel alignment is

```math
\mathrm{CKA}(X,Y)=
\frac{\|X_c^\top Y_c\|_F^2}
{\|X_c^\top X_c\|_F\,\|Y_c^\top Y_c\|_F}.
```

Feature dimensions may differ, but rows must describe the same samples. Linear CKA is unchanged by orthogonal coordinate transformations and positive uniform scaling, but generally changes under other invertible basis transformations. Test the estimator with identity, an orthogonal transform, uniform rescaling, changed pairing and zero-variance inputs. Declare a zero denominator undefined or refuse the calculation. High similarity alone cannot establish equivalent predictions or matching mechanisms. [Kornblith et al.](https://arxiv.org/abs/1905.00414v4)

Treat whitened or otherwise transformed similarity as a separately defined diagnostic. Pin its covariance sample, regularisation and numerical domain. It can suppress meaningful dominant directions or amplify low-variance noise. Keep the untransformed result beside it and preserve the study's own variant definition; a local metric should remain identified as such.

## Compare reconstruction at a stated sparsity

Identify encoder, decoder, biases, activation function, objective and auxiliary losses. For shared latents, state which inputs produce them and which model-specific decoder reconstructs each representation. Measure each side separately so pooled error cannot hide a failed model.

Distinguish per-sample TopK from batch-level selection. For B samples, BatchTopK retains Bk activations across the batch; individual samples can receive different numbers of active latents. Specify whether ranking uses raw or scaled activations, positivity and tie rules, and the separate inference rule. A calibrated inference threshold differs from the training mask. Compare achieved sparsity and downstream effects as well as nominal k or architecture name. [Bussmann et al.](https://arxiv.org/abs/2412.06410v1)

Fix the reconstruction denominator as well as the numerator. For example, with a reference mean fitted on a permitted training or calibration population:

```math
R=\frac{\sum_i\|x_i-\hat{x}_i\|_2^2}
        {\sum_i\|x_i-\mu_{ref}\|_2^2}.
```

Call this the study's chosen normalised reconstruction score. The label FVU alone leaves mean, centring, weighting and reduction unspecified. Use consistent masks and normalisations across comparable alternatives. Test perfect reconstruction, a mean-only predictor, error worse than the reference and a zero denominator. Keep values above one visible. Report by model, layer and relevant sample class, with paired uncertainty calculated at the appropriate unit.

Compare per-model, transferred and shared dictionaries where they answer the claim. Identify actual dictionary width, distribution of active latents, training exposure, compute and seeds. Larger capacity or a different token population can explain lower error without establishing better factorisation at the claimed resource match.

## Check features beyond reconstruction

A decoder norm ratio can suggest model specificity but cannot establish it. A sparse objective may suppress one decoder while signal remains in its residual, or represent a shared concept through different latent combinations. Use an independently justified presence diagnostic, such as the chosen Latent Scaling procedure, and inspect residual and reconstruction evidence. Compare objectives or sparsity mechanisms relevant to this failure. Improvement on another model pair suggests a method to test, not a result for this study. [Minder et al.](https://arxiv.org/abs/2504.02922v4)

Match features across seeds using declared evidence. Decoder direction, activation correlation and overlap of top examples answer different questions. Specify the matching algorithm, normalisation, thresholds and treatment of unmatched, split or merged features. Similar reconstruction can coexist with unstable individual latents. Support labels with positive and counterexamples, held-out behaviour and uncertainty. A plausible generated name is not evidence of a mechanism.

For intervention claims, use the shared comparison method. Keep the hook, replacement, control, outcome and affected population clear. An ablation score alone does not establish causality. Distinguish an expert's review of a label from behavioural evidence in an experiment, and limit high-stakes claims to actual validation and permitted use.

In the study review, return the paired inputs, mathematical definition and numerical controls, comparable reconstruction and sparsity results, feature stability and interpretation limits. Identify missing original artifacts. Extraction, training and computation use the actual library; model installation, trained artifacts and runtime connections need their own provision.

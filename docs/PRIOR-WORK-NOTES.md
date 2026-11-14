# Prior-work notes (search log, November 13, 2026)

**What this is.** A log of web searches made by the local seat while the experiments ran. It is a
first pass over secondary summaries and abstracts; the papers were not read in full. It is not a
priority audit, and it supports no claim of being first.

## Searches and what came back

| Topic searched | Closest work returned | How it relates |
|---|---|---|
| Pruning count tables with backoff | Stolcke, *Entropy-based Pruning of Backoff Language Models* (2000), https://arxiv.org/abs/cs/0006025 | Same family as count-guided cuts with fallback to the box table. A production model was cut to 26% of its size with no loss in recognition. |
| One global budget against per-layer budgets | Lee et al., LAMP, https://arxiv.org/abs/2010.07611 ; Zhou et al., https://arxiv.org/abs/2105.01571 | Same family as EQUAL / SEALED / OPEN. Reported that a raw global threshold can empty some layers at high sparsity; OPEN here also leaves some tables with nothing at 1/16 and beyond. |
| Pruning and merging experts in mixture-of-experts models | TENP https://arxiv.org/pdf/2606.09885 ; MAESTRO https://arxiv.org/pdf/2607.08601 ; REAM https://arxiv.org/pdf/2604.04356 ; MoNE https://arxiv.org/pdf/2507.00390 | Same family as FUSE (removing or merging whole experts). MAESTRO argues expert importance must account for routing; REAM merges instead of deleting. |
| The same model at several sparsity levels, with fallback | Pruned early-exit networks https://arxiv.org/pdf/2207.03644 ; a dense-fallback diagnostic in dense-to-sparse LLM training https://arxiv.org/pdf/2606.10722 ; DRESS sparse subnets (Meta research) | Related ideas: sparse submodels selected at run time, dense fallback as a diagnostic. No result described a stack of one network at alternating densities with pass-through of removed entries. |
| One model's routing applied to another | Teacher-guided routing (CVPR 2026 supplement); B-Distill (AAAI); MoE routing testbed https://arxiv.org/pdf/2604.07030 | Routing distilled from a teacher into a student during training. No result described a layer at one density choosing the table for a layer at another density at run time, with no training. |
| Small targeted removals with sham controls | Pruning a small weight set reverses ROME/MEMIT edits https://arxiv.org/html/2605.28839v1 ; CrispEdit https://arxiv.org/abs/2602.15823 | Small targeted edits are established. The results did not mention random-edit or wrong-place sham controls. |
| Weight-space programming | Takehi, Teoh, Zhang, Khattab, *Sparks of Weight-Space Programming* (2026-11-02), via https://paperswithcode.co/paper/131739 ; critique https://arxiv.org/html/2611.08070v1 | The guide for the surgery panel (read, diagnose, edit a very small share). The critique asks for controls against familiarity; the shams here answer to that request. |

## Reading

- The single ingredients (count-guided pruning with backoff, budget allocation across parts,
  removing or merging experts, small targeted edits) each have published relatives.
- These searches returned nothing describing (a) layers of one network at alternating extreme
  densities with the signal passing through, or (b) run-time steering of one density's table
  choice by another density, or the measured asymmetry between guiding and answering.
- "Returned nothing" is the result of six searches. A proper audit would read the listed papers
  in full, search the compression literature (context mixing, PAQ-style model switching) directly,
  and be checked by a second seat.

## Added 2026-11-14 (searches of that day; abstracts only, English)

- **Autapses** (a neuron's synapse onto itself): reported in most neocortical pyramidal cells studied (Lubke et al. 1996 and later work). V51 uses the word for a delayed copy of a switch's own choice; nothing is claimed about biological neurons, and hysteresis in switching is a known device.
- **Stickiness of a switch** (V60): the penalty log((K-1)(1-a)/a) for a source that changes with rate a is standard: Herbster and Warmuth, *Tracking the Best Expert* (1998); the Viterbi path of a switching model.
- **Gated linear mixing** (V59): the logistic mixer of PAQ-style compressors (Mahoney); gated linear networks (Veness et al.).
- **Helper memories beside a fixed model** (V61): cache language models (Kuhn and De Mori 1990); secondary estimation tables in PAQ-style coders; nearest-neighbour language models (Khandelwal et al. 2020; store pruned by 95%, He et al. 2021); parameters in external tables (Baykal et al. 2023); neuron-astrocyte associative memory (Kozachkov, Slotine, Krotov 2023).
- **Not found in those searches:** helper capacity measured against the density of the layer it serves; a comparison of where the helper sits (per unit, per group, pooled, shared values against shared steps). A search that finds nothing is weak evidence.
- **Affine-parameter replay** (Matrix Law Essentials PR #13, reviewed by this seat): the identities are the offline-online decomposition of reduced basis methods (Prud'homme et al. 2002; Rozza, Huynh, Patera 2008).


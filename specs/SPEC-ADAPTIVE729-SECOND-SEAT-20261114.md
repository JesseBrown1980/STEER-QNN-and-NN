# Independently frozen control — same-order adaptive model, no STEER network

**Seal date:** 2026-11-14. This specification is committed before its Rust
program and before the requested evaluation on the private 2.69 GB stream.
Refutation of claims A/B/C in STEER issue #1 is the objective; this
control addresses **A**, the strongest remaining vulnerability of the referee.

## Question and original source facts (read before the freeze)

Do V50/V57's gains (approximately 8.6% stream-order and 8% shuffled)
reflect ordinary online adaptation rather than a special property of
729 tables and their referee? V57's \`N_LEARN_PLAIN\` under
\`BOX_PLAIN\` stores approximately 5.27M–5.57M corrected entries across
the four exam splits. Original, exact scored bytes:

| Split | V57 N_STATIC_PLAIN | V57 N_LEARN_PLAIN |
|---|---:|---:|
| TEXT_1_NATURAL | 1,100,851 | 1,006,118 |
| TEXT_1_SHUFFLED | [read result row] | 1,037,245 |
| TEXT_2_NATURAL | [read result row] | 993,016 |
| TEXT_2_SHUFFLED | [read result row] | 1,025,922 |

The complete frozen comparison MUST read all four exact
\`N_STATIC_PLAIN\` and \`N_LEARN_PLAIN\` bytes directly from the
archived \`results/RESULT-COMBINED729-V57.hbp\`; a hand-transcribed
reference is never authoritative.

## Inputs, scoring and units

Same full stream, same 19,683 blocks, same exact block-size rule
\`block=file_size/19683\`, same 2,048-byte held-out tail of every block
excluded from global training, same test blocks \`j mod 9=1\` and
\`j mod 9=7\`, same natural and
\`sort by (h16(j xor 0x2545f491),j)\` shuffled order.
Within each test text, context is continuous across tail boundaries,
exactly as in the original V57 read path.

Each bit is read MSB first. Context is exactly the previous **two
bytes** plus the current byte's binary bit-tree node (values 1..255).
Initialize the previous two bytes to zero at the start of each text;
do not reset at tail boundaries. State transitions are after scoring,
with no access to the current bit before prediction.

Use the exact \`pressure_boxes::cost_table()\`: integer 1/256-bit units,
\`bit=1\` costs \`cost[p]\`; \`bit=0\` costs \`cost[4096-p]\`.
Whole-text bytes are \`(total_cost+1024)/2048\` with exact integers.
A self-test must cross-check cost of 2048 and the bit order with
a separately implemented synthetic byte-by-byte reference.

## Independent model and finite cap

One global order-2 binary context tree. No 27 boxes, no 729-table
selection, no guide, no density/pressure layer, no autapse, no mix,
no QNN-specific state. The single context key ranges over
\`65536*256\` possibilities. The model may only update an entry
once the bit has been scored.

The **same-online-rate** arm uses V57's exact fixed step r=4,
\`target=4095\` for one and \`target=1\` for zero:
\`p' = clamp(p+((target-p)>>4),1,4095)\`.
Use the same signed arithmetic shift semantics as Rust's original
\`toward\`. A single baseline overlay has at most **5,800,000**
distinct changed context keys, a conservative cap just above the
largest V57 online update count reported (5,707,772).
No silent eviction, replacement, learning outside the cap or
switch to approximate hashing. If cap is hit and an update to a new
key is needed, the result is \`RESOURCE_HOLD\` (report the
number of such attempts and costs, but make no matched-cap verdict).
Record the final count of unique updated positions, the cap,
the context key space and the memory use of the implementation.
A cap of 5.8M update positions is not necessarily the same physical
RAM as V57's multiple HashMaps; the report must distinguish
*learned degrees of freedom* from measured bytes of storage.

## Four mandatory arms for each of four exam cases

1. \`COLD_STATIC\`: every prediction is 2048 (fair-coin; no updates).
2. \`COLD_ADAPT_R4\`: start p=2048 for all contexts, online rate r=4.
3. \`TRAIN_STATIC\`: initialize each context from a **single global
   order-2 count table** trained on all block prefixes (never on
   any 2,048-byte tail), with the same \`prob(a,b)\` integer estimator
   as the pretrained STEER tables; hold fixed at test.
4. \`TRAIN_ADAPT_R4\`: same global pretrained estimator plus online
   update rate r=4.

Training must be corpus-global, not conditioned on block id, box,
density or held-out evaluation text. Exclude every block tail,
including those of test blocks. Pretraining may reset the prior two
bytes per block (same as STEER's training counter). No access to test
tail bytes or their order while fitting. Pretraining counts cover
all 16,777,216 order-2 byte/tree context keys, but online overlays
are limited by the cap above. Count model storage and online
degrees of freedom separately.

To avoid a confound, the full stream's pretraining pass must be
identical for all four test cases. Test-time adaptive state is reset
at the **start of each complete case**, not per block and not shared
across cases. The online arms must not see preceding test cases.
Both static arms are required to detect an implementation or
training mistake. Run two synthetic tests: the true bit must only
affect later predictions, and the identical byte sequence with
boundaries rearranged must produce the same result when the stream of
bytes is identical.

## Presealed judgements and negative controls

- Report exact bytes for all four arms for all four cases.
- Compare against the exact corresponding V57 \`N_STATIC_PLAIN\`
  and \`N_LEARN_PLAIN\` **using the existing row file**, without
  rerunning, altering or training its QNN.
- \`PLAIN_WINS\` on a case if \`TRAIN_ADAPT_R4\` uses fewer bytes than
  \`N_LEARN_PLAIN\` at that case. Also report the \`COLD_ADAPT_R4\`
  head-to-head result. Require all four wins for a robust claim;
  any partial wins must be reported honestly.
- A stronger refutation of *QNN-specific online adaptation* would be
  a pretrained plain model that matches or beats the QNN across both
  natural and shuffled orders on both texts, with comparable online
  changed-entry counts. Report the observed cap utilization.
- If plain model loses, that **does not by itself show** a distinct
  advantage of the QNN architecture: it has one global trained table
  compared with 729 specialized inner tables and could be
  underparameterized before the exam. Further capacity-matched
  multiple-independent-model controls may be needed.
- A static-to-adaptive improvement alone is ordinary adaptation.
  The evidence for a *QNN-specific* gain is a margin against the
  appropriate adaptive plain baseline, not an 8% gain against a
  frozen QNN.

**Sealed numerical predictions:** none. The attack is deliberately
falsifiable and symmetric; thresholds and comparisons are frozen,
while all unknown plain-model bytes remain unknown until the run.

## Instrument checks before any decision

1. Freeze the existing V57 file by exact Git blob/sha256. Verify
   exactly four \`BOX_PLAIN,N_LEARN_PLAIN\` and four
   \`BOX_PLAIN,N_STATIC_PLAIN\` rows, with four distinct cases.
2. Check all four cases' total bytes = 2,187 tails × 2,048
   = 4,478,976. Shuffled and natural contain the same tails.
3. Require a passing synthetic reference that independently computes
   order-2 contexts, bit costs and online update order.
4. Require pretraining to exclude 2,048 bytes per block and reset
   context at block boundaries; forbid learning from test tails
   during pretraining.
5. Require nonnegative total costs, valid probability range
   1..4095, no silent overlay overflow, no floating point, no
   unsafe, Rust 1.81, strict clippy.
6. Recheck source/prediction commit chronology. The *source* must
   be committed after this spec, and the real stream data must
   be introduced as a separately verifiable input.

## Scope

The author's frozen mixer V62 has a distinct question and is left
unchanged. This spec is an independent **second-seat control
experiment** for STEER issue #1, not a rejudgement of V62 or a
claim that results are already measured. Until the 2.69GB
stream has actually been run, report \`REAL_STREAM=NOT_RUN\`,
\`REFUTATION=UNDETERMINED\`. Synthetic tests are instrument
checks, not substitute experimental results.

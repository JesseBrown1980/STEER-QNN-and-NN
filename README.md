# STEER: density, steering and surgery on a table network

**Status, November 13, 2026.** Twelve panels are measured and judged, five more are sealed and
running or queued. Everything is integer Rust 1.81. Every panel's predictions were committed
before its program ran, and every judgement lists the misses. One stream, one kind of network,
no second-seat review yet, and only a first literature search: this is an original framework with
early measurements, not a claimed discovery.

## What this is

A network of 729 small prediction tables, trained on 2.69 GB of Asolaria material, is edited
directly (no training loop) and tested on held-out text. The score is how small the held-out text
codes: fewer bytes is better. The questions:

1. How concentrated is the weight, and how hard can the network be cut? (density)
2. What happens when the same network runs at very different densities, and one layer guides
   another? (steering)
3. Can a few entries be found and removed so that the network gets better? (surgery)
4. Do loops, memory and correction from the revealed answer help, and how? (loops and learning)

The density layers (normal, pressurized, neutron star, black hole), the loops, the neuron with
three loops, growth from a dense core and maturing cells that teach are the operator's ideas.
Editing stored weights directly follows *Sparks of Weight-Space Programming* (Takehi, Teoh, Zhang,
Khattab, 2026-11-02). Pressure, flow, walls, stars and neurons are analogies on a table network.

## The network

- **Outer network:** 27 box tables. **Inner network:** 729 tables, 27 per box, 75,696,165 entries.
- **Tables:** order-2 bit-tree counts; an entry's weight is how often it was seen.
- **Guide:** a switch picks the box, then the inner table, by cheapest cost over recent bytes.
- **Held-out text:** the last 2,048 bytes of every block were never trained on; a split is 2,187
  of those tails (4,478,976 bytes).

## Map of the panels

| Panel | Question | Result | Predictions met | Files |
|---|---|---|---|---|
| V39 | Density cuts, flow rules, layer stacks (first run, Node) | Weight is concentrated; flow helps; hourglass returns | 5 of 8 | `JUDGE-PRESSURE729-V39` |
| V40 | The same in integer Rust; one layer steering the next | All 62 arms identical to V39; a dense layer steered by a lighter one improves; the layer after it pays 13% | 4 of 6 | `JUDGE-PRESSURE729-V40` |
| V41 | Do the results repeat on two other held-out texts? | **Replicated**, within hundredths of a percent | 6 of 6 | `JUDGE-PRESSURE729-V41` |
| V42 | Surgery: diagnose on one text, remove entries, judge on others | Removing 7,570 entries (1 in 10,000) gains **1.4%**; shams do nothing or hurt | 7 of 9 | `JUDGE-SURGERY729-V42` |
| V43 | Blocks the network never trained on | Every pattern carries; no penalty (0.1%) | 4 of 6 | `JUDGE-PRESSURE729-V43` |
| V44 | Longer guide memories, blending, rule-based cuts | Box memory 512 gains about 5% **in stream order** (see V46) | 4 of 6 | `JUDGE-TUNING729-V44`, `NOTE-V44-READ-WITH-V46` |
| V45 | The law: does an ungated loop go bad? | **Not shown in the predicted form**; six rounds kept helping (2.3%, 1.9%); the loop's own view is about twice too good | 2 of 5 | `JUDGE-RECURRENCE729-V45`, [`LAW.md`](LAW.md) |
| V46 | Still longer box memories, with a shuffled-order control | The whole box-memory gain is adjacency of related text; on unrelated pieces 512 costs **+11%** | 3 of 5 | `JUDGE-BOXGUIDE729-V46` |
| V47 | A lazy guide that rescans only when unsure | **Not shown**: neither cheap nor close to the full guide | 1 of 4 | `JUDGE-LAZYGUIDE729-V47` |
| V48 | Side loops with feedback to the sender (untrained) | **Not shown**; the denser the loop, the more its feedback costs | 4 of 6 | `JUDGE-LOOPS729-V48` |
| V49 | A cell with three loops, side by side and in a row | Box neuron gains 4% in stream order (6% with an inner neuron) but costs 4.6% shuffled; dense loops add 0.5% under a good box guide | 4 of 10 | `JUDGE-NEURON729-V49` |
| V50 | Sensing, memory, and a referee that corrects the values | **Gains 8.6% in stream order and 8.0% shuffled**; each loop's ablation hurts; density-ordered correction rates not shown | 4 of 5 | `JUDGE-THREELOOP729-V50` |
| V51 | Autapses: a cell's own earlier choice returning after a delay | sealed, running | – | `SPEC-AUTAPSE729-V51` |
| V52 | The same loops with trained weights | sealed, queued | – | `SPEC-TRAINED729-V52` |
| V53 | Growth from a black-hole core that learns as it reads | sealed, queued | – | `SPEC-GROWTH729-V53` |
| V54 | Read the weights (information against the box; nearest sibling), then edit | sealed, queued | – | `SPEC-READEDIT729-V54` |
| V55 | Maturing cells that teach their neighbours | sealed, queued | – | `SPEC-MATURE729-V55` |

Specs are in `specs/`, rows and judgements in `results/`.

## What has been measured

### Density

| Heaviest share of entries | 1/2 | 1/4 | 1/8 | 1/16 | 1/64 | 1/256 |
|---|---|---|---|---|---|---|
| Share of total weight | 99.5% | 98.6% | 96.3% | 90.2% | 65.0% | 34.2% |

Cost of cutting the inner network, weight allowed to flow between tables, box table as fallback
(three held-out texts and one of unseen blocks):

| Density kept | 1/8 | 1/16 | 1/256 |
|---|---|---|---|
| Cost against the full network | +0.20% to +0.28% | +1.39% to +1.46% | +11.8% to +12.4% |

Random cuts to 1/2 cost 9%. Without the box fallback the same cuts cost several times more.

### Steering

- A layer cut to 1/256 that is **guided** by a full layer answers within 0.2% of the full network
  (with pass-through). The same layer as the **guide** of a full layer costs 23%.
- So the guide must be a full layer; the answering layer can be cut.

### Surgery

| Entries removed (found on an exam text) | Two verdict texts |
|---|---|
| 1,000 most harmful (0.0013%) | −1.1%, −1.0% |
| 7,570 most harmful (0.01%) | −1.4%, −1.4% |
| All 1,437,697 that looked harmful | −0.6%, −0.6% |
| Random sham, wrong-table sham (75,696 each) | +0.3%, +0.5% |
| Six rounds of 7,570 (V45) | −2.3%, −1.9% |

About a quarter of the gain seen on the exam text carries over. A rule read from the tables alone
(few observations, far from the box value) did not reproduce it.

### The guide and its memory

- A longer memory for the switch that chooses the box gains about 5% when the text is read in
  stream order, and costs 11% or more on unrelated pieces. The gain is the adjacency of related
  text, which a stream has.
- A box cell that votes among three memories keeps a 4% gain in stream order and halves the
  penalty on unrelated pieces (4.6%). No guide tested so far is good in both.
- With an oracle that is handed the right inner table, 5% to 6% more is available.

### Loops and learning

- **A referee that corrects the stored values after each prediction is the largest robust gain:**
  about 8.6% in stream order and 8.0% on unrelated pieces. Removing the memory loop costs 5% to
  9%; removing the sensing is catastrophic.
- Untrained side loops did not help; feedback from layers that know less than the sender makes
  the sender worse, and three such layers agreeing is not evidence.
- Dense loops (long memory on the densest loop) add about 0.5% under a good box guide. Measured
  once, not predicted, not yet replicated.

## Corrections kept on record

- **V44 must be read with V46.** "Patient A" (−5.6%) and "Patient B" (−4.4% with 24% of the
  entries) hold for stream-order text only.
- **V45 did not show the law** in the form predicted; `LAW.md` carries that beside the statement.
- **Seat predictions miss often** (48 of 76 met so far). They are sealed first for that reason.

## Nearest published work

A search log, not an audit: [`docs/PRIOR-WORK-NOTES.md`](docs/PRIOR-WORK-NOTES.md).

- Count-guided cuts with fallback: Stolcke, *Entropy-based Pruning of Backoff Language Models*
  (2000). Budget flow: global against per-layer pruning (LAMP; global sparsity constraint).
- Removing or merging experts: several 2026 papers on mixture-of-experts models.
- Correcting values from the revealed answer, with a step that shrinks with age: standard in
  adaptive data compression.
- Small targeted weight edits: model editing, and the weight-space programming paper.
- Not found in that search: one network at alternating extreme densities with pass-through; one
  density steering another's choice at run time; loops of different sizes paired with densities.

## Reproduce

```
cargo build --release
./target/release/pressure729 <stream> [remainder] [unseen]   # V40, V41, V43
./target/release/surgery729  <stream>                         # V42
./target/release/tuning729   <stream>                         # V44
./target/release/recurrence729 <stream>                       # V45
./target/release/boxguide729 <stream>                         # V46
./target/release/lazy729 | loops729 | neuron729 | threeloop729 <stream>   # V47 to V50
```

Each run takes 10 to 60 minutes and about 2 GB of memory. The stream (2,692,693,942 bytes of
Asolaria files with duplicates dropped) stays on the operator's machine, so the hosted checks
cover the code, the no-float rule, the format, clippy and ten tests on a small synthetic stream;
the full-stream rows are from local runs and are filed with their commit.

## Scope

- One stream, one set of tables, count tables and not a trained neural network.
- Stream-order results rely on related text being adjacent; shuffled-order results are given
  beside them from V46 on.
- Exam and verdict texts come from the same stream, from different blocks.
- No second seat has reviewed any of it.
- The first run (V39) used a Node program with a float-built cost table; the integer program
  reproduces it exactly and the integer rows are the record.
- The big-integer module is copied unchanged from the QNN repository (git blob `23f5447e`).
- Related: GPT-6's hosted 81-node pressure/box classifier experiment in
  `JesseBrown1980/GGUF-files-operating-as-trained-quantum-neural-network-models`.

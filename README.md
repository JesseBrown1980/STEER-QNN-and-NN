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
| V51 | Autapses: a cell's own earlier choice returning after a delay | A box cell with autapses of strength 4,096 gains **4.0% in stream order and 1.5%, 0.9% shuffled**: the first guide good in both; three delay sizes not shown | 4 of 5 | `JUDGE-AUTAPSE729-V51` |
| V52 | The same loops with trained weights | Training adds little (0.9% against 0.7% untrained); dense loops earn weight | 2 of 5 | `JUDGE-TRAINED729-V52` |
| V53 | Growth from a black-hole core that learns as it reads | From the 1/256 core: 1.3% better than the fixed full network; from the full network 8.6%; from nothing about 0.5% | 3 of 5 | `JUDGE-GROWTH729-V53` |
| V54 | Read the weights (information against the box; nearest sibling), then edit | Cutting by information: 1/16 costs **+0.1%** (by weight +1.4%), 1/64 +1.3%, 1/256 +4.6%; hand edits of trained weights all worse | 4 of 5; 1 of 5 | `JUDGE-READEDIT729-V54`, `JUDGE-HANDEDIT729-V54E3` |
| V55 | Maturing cells that teach their neighbours | Helps a young network (2.2%); **hurts a trained one** (4.4% where the fixed step gives 8.6%); adult neighbours as teachers hurt | 2 of 6 | `JUDGE-MATURE729-V55` |
| V56 | Stronger autapses | Stronger is not better: 8,192 costs 3.4% shuffled, 32,768 and above collapse | 2 of 4 | `JUDGE-STRONGAUTAPSE729-V56` |
| V57 | Autapse guide, referee and information cores in one network | Together **10.2% in stream order, 7.5% and 7.0% shuffled**; the gains overlap by about 3.5 points; a 1/16 core with the referee equals the full network | 3 of 6 | `JUDGE-COMBINED729-V57` |
| V58 | A fast start with a floor on the correction step | **Shown:** 0.9% better than the fixed step on the full network, 1.7% to 2.1% from the 1/256 core, in both orders | 4 of 4 | `JUDGE-FLOOR729-V58` |
| V59 | A trained gated mixer in place of the hard switch | **The guide/answer asymmetry holds in a trained model**; the mix gains 4% to 5%; trained-then-frozen weights are far worse (cause open) | 4 of 6 | `JUDGE-MIXER729-V59` |
| V60 | Measure the coherence length L; predict the stickiness from L alone | **Formula not shown** (optimum 2 to 3.5 times higher); direction shown. One stickiness on the held box beats the three-delay autapse: **5.4% in stream order, 3.3% and 3.0% shuffled** | 3 of 5 | `JUDGE-COHERENCE729-V60` |
| V61 | Bounded helper stores beside the cells (block, sphere, cube, path, step) | sealed, queued | – | `SPEC-HELPER729-V61` and addendum 1 |
| V62 | Is a trained-then-frozen mixer bad because of the snapshot or a fault? | sealed; program not yet written | – | `SPEC-FROZEN729-V62` |
| V63 | The strongest results on test texts that share no factor with the powers of three (`j mod 11`; then `j mod 27` against `j mod 29`) | **Carry over:** referee 8.8% and 8.2% on mod 11; on 27 against 29 the referee agrees to 0.02 points (7.20% and 7.22%) and the 1/16 core holds | 3 of 3; 4 of 4; 3 of 4 | `JUDGE-PRIMESPLIT729-V63`, `-A1`, `-A2` |
| V64 | A formula from the trime/prime ladder (3/5, 9/11, 27/29) | **No curve in spacing for the guide** (scatter of about a point between texts). The referee's gain rises with length and levels off: about `9766 n/(n+248)` per 100,000 in stream order, n in tails; shape not distinguished from a logarithm | 0 of 2 judged | `JUDGE-LADDERFORMULA729-V64` |

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
  penalty on unrelated pieces (4.6%).
- **A box cell whose own earlier choice returns to it (an autapse, strength 4,096) is good in
  both orders:** 4.0% in stream order, 1.5% and 0.9% on unrelated pieces. The evidence window
  stays short, so a real change of text still overturns the choice. Stronger is worse (V56).
- **A single stickiness on the box held a byte ago is a better guide than the three delays**
  (V60): 5.4% and 5.3% in stream order at strength 16,384 (the largest tried, still improving),
  3.3% and 3.0% on unrelated pieces at 8,192.
- The best box stays the best for about 13,500 to 14,700 bytes in stream order and about 2,100
  shuffled (V60). The conjectured formula for the stickiness, `256 log2(26 L)`, gets the
  direction right and the size wrong by a factor of two or more.
- With an oracle that is handed the right inner table, 5% to 6% more is available.

### Loops and learning

- **A referee that corrects the stored values after each prediction is the largest robust gain:**
  about 8.6% in stream order and 8.0% on unrelated pieces. Removing the memory loop costs 5% to
  9%; removing the sensing is catastrophic.
- Untrained side loops did not help; feedback from layers that know less than the sender makes
  the sender worse, and three such layers agreeing is not evidence.
- Dense loops (long memory on the densest loop) add about 0.5% under a good box guide. Measured
  once, not predicted, not yet replicated.
- **Gains overlap.** Autapse guide alone 5.0%, referee alone 8.6%, both together 10.2% (stream
  order); 2.3%/1.7%, 8.0%, 7.5%/7.0% shuffled. The inner autapse costs once the referee is on.
- **The referee is ordinary adaptation applied to this network** (second-seat control, PR #3,
  run on the stream 2026-11-14). A plain order-2 model with no boxes gains 24% from the same
  correction rule in stream order (the network: 8.6%) and 6.0% shuffled (the network: 8.0%).
  The plain model never beats the network: it needs 27% more bytes in stream order and 53%
  more shuffled. Capacity is not matched (one table against 756), so this bounds the claim and
  does not close it.
- **A cell should start fast and keep a steady step** (V58): 1/4 on its first corrections,
  halving to a floor of 1/16.
- **Slowing or fixing what has been learned hurts:** maturing cells (V55), hand edits (V54),
  frozen mixer weights (V59).
- **Cutting by information, not weight:** a 1/16 core with the referee equals the full network
  with the referee in both orders (V57).

### A trained combination (V59)

| Against the hard switch | Stream order | Shuffled |
|---|---|---|
| Mixer gated by the full guide | −5.1%, −5.0% | −4.3%, −4.2% |
| Mixer, one weight set, no gate | −3.2% | −1.9%, −1.8% |
| Mixer gated by a crushed (1/256) guide | −2.7%, −2.6% | −0.8%, −0.7% |
| Crushed answerers, either gate | +10.1% to +10.6% | +9.4% to +9.8% |

A crushed guide costs 2.4 to 3.6 points over full answerers and nothing over crushed ones: the
guide is worth what there is to choose between. The mixer is the established one of PAQ-style
compressors; the measurement is what is reported here.

## Corrections kept on record

- **V44 must be read with V46.** "Patient A" (−5.6%) and "Patient B" (−4.4% with 24% of the
  entries) hold for stream-order text only.
- **V45 did not show the law** in the form predicted; `LAW.md` carries that beside the statement.
- **Seat predictions miss often** (90 of 145 met through V64). They are sealed first for that
  reason.
- **The seat's reading of the trained weights failed its test** (V54 hand edits: 1 of 5).
- **The stickiness formula was a coincidence** (V60): 4,019 against 4,096 compared the
  formula with a different device; on the device it describes the optimum is 8,192 and above.
- **Gains were expected to add and do not** (V57): the seat said about 12% and 9% before reading
  the result; the measurement is 10.2% and about 7.3%.

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
./target/release/autapse729 | trained729 | growth729 | read729 | handedit729 | mature729 | strong729 <stream>   # V51 to V56
./target/release/combined729 | floor729 | mixer729 | coherence729 | helper729 <stream>   # V57 to V61
```

Each run takes 10 to 60 minutes and 2 to 4 GB of memory; runs that keep learned entries are
run one at a time. The stream (2,692,693,942 bytes of
Asolaria files with duplicates dropped) stays on the operator's machine, so the hosted checks
cover the code, the no-float rule, the format, clippy and fourteen tests on a small synthetic stream;
the full-stream rows are from local runs and are filed with their commit.

## Scope

- One stream, one set of tables, count tables and not a trained neural network.
- Stream-order results rely on related text being adjacent; shuffled-order results are given
  beside them from V46 on.
- Exam and verdict texts come from the same stream, from different blocks.
- The network is built on powers of three and the test texts through V61 are blocks with
  `j mod 9` equal to a remainder, so each takes the same three positions inside every table.
  V63 repeats the strongest results on `j mod 11`, small scale; one prime, one remainder.
- No second seat has reviewed any of it.
- The first run (V39) used a Node program with a float-built cost table; the integer program
  reproduces it exactly and the integer rows are the record.
- The big-integer module is copied unchanged from the QNN repository (git blob `23f5447e`).
- Related: GPT-6's hosted 81-node pressure/box classifier experiment in
  `JesseBrown1980/GGUF-files-operating-as-trained-quantum-neural-network-models`.

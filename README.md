# STEER: pressure boxes and steered layer stacks on a table network

**Status, November 13, 2026: MEASURED on one stream and one split, integer Rust, predictions sealed
before each run. Not yet reviewed by a second seat. A first literature search was made; it is not a
priority audit.**

A network of 729 small prediction tables, trained on 2.69 GB of Asolaria material, is edited
directly (no training loop) and tested on 4,478,976 bytes of held-out text. The score is how small
the held-out text codes: fewer bytes is better. Three questions:

1. How concentrated is the weight, and how hard can the network be cut?
2. Does it help to let weight flow between tables, inside sealed boxes or with the walls open?
3. What happens when the signal passes through layers of the same network at very different
   densities, and when each layer steers the next?

The idea of the layers (normal, pressurized, neutron star, black hole, and back) is the operator's.
Editing stored weights directly follows *Sparks of Weight-Space Programming* (Takehi, Teoh, Zhang,
Khattab, 2026-11-02). Pressure, flow and walls are analogies on a table network; no fluid solver runs.

## The network

- **Outer network:** 27 box tables. **Inner network:** 729 tables, 27 per box.
- **Tables:** order-2 bit-tree counts; entries seen once are dropped. 75,696,165 inner entries.
- **Weight of an entry:** how often it was seen.
- **Switch:** the box is chosen by cheapest cost over the previous 64 bytes, then the inner table
  the same way among the 27 of that box.

## 1. Weight concentration

| Heaviest share of entries | 1/2 | 1/4 | 1/8 | 1/16 | 1/64 | 1/256 |
|---|---|---|---|---|---|---|
| Share of total weight | 99.5% | 98.6% | 96.3% | 90.2% | 65.0% | 34.2% |

The median entry was seen 5 times.

## 2. Density and flow

Density is the share of inner entries kept; the total kept is the same for every rule at one density.

- **EQUAL:** each table keeps its own share (no flow).
- **SEALED:** one threshold per box; weight flows between the 27 tables of a box.
- **OPEN:** one threshold for all 729 tables.
- **RANDOM** and **INVERSE** (the lightest entries kept) are controls.
- A removed entry falls back to the box table (PARENT) or to probability 1/2 (FLAT).

Coded size against the full inner network (1,083,359 bytes), PARENT fallback:

| Density | EQUAL | SEALED | OPEN | RANDOM |
|---|---|---|---|---|
| 1/2 | 0.0% | −0.1% | −0.1% | +9.4% |
| 1/4 | +0.6% | +0.1% | −0.1% | +13.6% |
| 1/8 | +2.7% | +1.5% | +0.2% | +15.5% |
| 1/16 | +5.1% | +3.8% | +1.4% | +16.2% |
| 1/64 | +11.0% | +9.7% | +7.1% | +16.4% |
| 1/256 | +14.3% | +13.4% | +12.4% | +16.0% |

With FLAT fallback the same cuts cost far more: at 1/16, +57% (EQUAL), +51% (SEALED), +18% (OPEN).
All 62 arms are in `results/RESULT-PRESSURE729-V40.hbp`.

## 3. Layers at different densities

NORMAL = everything, PRESSURIZED = 1/4, NEUTRON STAR = 1/16, BLACK HOLE = 1/256, each sealed in its
box. The prediction for a byte (the light) enters a layer; where the layer's chosen table still
has the entry it replaces the incoming prediction, and where the entry was removed the light
passes through unchanged.

**FREE:** every layer chooses its own table. **STEER-k:** a layer may choose only among the k
tables the layer before it ranks cheapest; with k = 1 it is steered entirely by the layer before.

Bytes after each layer of the hourglass:

| Layer | Density | FREE | STEER-9 | STEER-3 | STEER-1 |
|---|---|---|---|---|---|
| 1 | NORMAL | 1,083,359 | 1,083,359 | 1,083,359 | 1,083,359 |
| 2 | PRESSURIZED | 1,084,984 | 1,084,642 | 1,083,883 | 1,083,359 |
| 3 | NEUTRON STAR | 1,092,592 | 1,090,655 | 1,086,910 | 1,084,247 |
| 4 | BLACK HOLE | 1,105,462 | 1,096,679 | 1,089,384 | 1,085,128 |
| 5 | NEUTRON STAR | 1,094,042 | 1,114,359 | 1,155,195 | 1,233,346 |
| 6 | NORMAL | 1,084,235 | 1,087,667 | 1,101,849 | 1,126,088 |

Share of bytes each layer sent to the right inner table (per thousand):

| Layer | Density | FREE | STEER-9 | STEER-3 | STEER-1 |
|---|---|---|---|---|---|
| 1 | NORMAL | 376 | 376 | 376 | 376 |
| 4 | BLACK HOLE | 134 | 171 | 227 | 319 |
| 5 | NEUTRON STAR | 320 | 274 | 201 | 131 |
| 6 | NORMAL | 373 | 365 | 345 | 318 |

What the table shows:

- **A dense layer steered by a lighter one does better than when free.** The black-hole layer
  improves from 1,105,462 to 1,085,128 as the steering tightens.
- **The cost falls on the layer after the black hole.** Steered by the black hole's choice, the
  neutron-star layer goes from 1,094,042 to 1,233,346 (+13%).
- **One more layer recovers most of it:** the final layer ends 0.3%, 1.6% and 3.9% above the free end.
- From the black hole outward (the OUTWARD stack) under STEER-1: 1,228,711 → 1,231,946 → 1,104,087
  → 1,092,761.

## Predictions

Every prediction was written and committed before its run. Misses are kept.

| Run | Met | Missed |
|---|---|---|
| V39 (first run, 8 predictions) | weight-guided beats random; PARENT beats FLAT; hourglass rises to the black hole and returns; outward stack ends at the normal value; the black-hole layer finds the right table less than half as often | the cut to 1/16 was predicted to cost over 10% and cost 0.8% over the plain baseline; OPEN was marginally above SEALED at 1/2; INVERSE was not the largest rule at 1/64 and 1/256 with PARENT |
| V40 (integer run and STEER, 6 predictions) | the integer run reproduces the first run (all 62 arms and 10 layer rows identical); the layer after the black hole is over 2% worse under STEER-1 (it is 13.7% worse); OUTWARD ends below HOURGLASS; the last layer's share matches a free neutron-star layer | STEER-1 end predicted 0.5% to 3% above free, measured 3.9%; STEER-9 predicted within 0.3% of free everywhere, measured up to 2.0% |

Judgements: `results/JUDGE-PRESSURE729-V39.hbp`, `results/JUDGE-PRESSURE729-V40.hbp`.

## Nearest published work found in a first search

- Pruning count-based language models with backoff to a lower-order model: Stolcke,
  *Entropy-based Pruning of Backoff Language Models* (2000), https://arxiv.org/abs/cs/0006025.
  Count-guided cuts with the PARENT fallback are of that family, and the finding that a few heavy
  entries carry almost all the weight agrees with it.
- One global budget against per-layer budgets in magnitude pruning: Lee et al., LAMP,
  https://arxiv.org/abs/2010.07611; Zhou et al., global sparsity constraint,
  https://arxiv.org/abs/2105.01571. EQUAL, SEALED and OPEN are of that family.
- For stacks of one network at alternating densities with pass-through of removed entries, and for
  one layer steering the next layer's table choice, that search returned hourglass architectures
  with dense layers only.

## Reproduce

```
cargo build --release
./target/release/pressure729 <stream file>     # about 11 minutes, about 2 GB of memory
```

The stream (2,692,693,942 bytes, 404,335 Asolaria files with duplicates dropped) stays on the
operator's machine, so the hosted checks cover the code, the no-float rule, and two tests on a
small synthetic stream; the full-stream rows are from the local run and are filed with the
program's hash.

## Scope and record

- One stream, one split, one switch window (64 bytes).
- The first run (V39) used a Node program whose cost table was built with floating logarithms.
  The integer program here computes the same table exactly and reproduces every V39 number, so the
  integer rows are the record and the V39 rows stay beside them as the first measurement.
- Order of events: V39 spec sealed locally (sha256 `c59ea357…`) before its run; V39 rows first
  published in draft pull request #34 of the QNN repository; V40 spec committed there as `0c28863`
  and the Rust program as `fc9d6a8`, both before the integer run.
- The big-integer module is copied unchanged from the QNN repository (git blob `23f5447e`).
- Related: GPT-6's hosted 81-node pressure/box classifier experiment in
  `JesseBrown1980/GGUF-files-operating-as-trained-quantum-neural-network-models`.

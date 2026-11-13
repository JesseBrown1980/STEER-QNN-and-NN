# LAW: Recurrence is mind — with a ground-truth corrective gate

**Status in this repository: NAMED; first test (V45) did not show it in the predicted form.** The law is the operator's, stated in
[N-Nest-Prime `LAW.md`](https://github.com/JesseBrown1980/N-Nest-Prime-INFINITE-SELF-REFLECT-AGENTS-NESTED/blob/main/LAW.md)
and tested in software in `asolaria-stone-to-moss` (October 15, 2026). Panel V45
(`specs/SPEC-RECURRENCE729-V45.hbp`) puts it to a measurable test on this repository's network.
This file will carry the result beside the statement, whichever way it falls.

## The law

> Recurrence + a ground-truth corrective gate = mind.

A loop that reads only its own output amplifies into hallucination. A loop corrected against real
input becomes cognition. Correction nests to any depth; consent does not: authorization to scale
anchors at the human, and a halt works at any level.

## What it means for work in this repository

1. **A diagnosis is not a verdict.** Any choice (an entry to remove, a window, a rule) is made on
   one held-out text and judged on another that took no part in the choice. A gain seen only on
   the text it was fitted to is reported as not carried over.
2. **A loop needs a gate it did not write.** A procedure that repeats (diagnose, edit, diagnose
   again) accepts a step only when an independent text agrees, and stops itself when it does not.
3. **A watcher must be able to see.** A check that cannot observe an item holds it; it does not
   pass it. In the layer experiments a crushed layer made a poor guide for the same reason.
4. **Every edit has a sham.** The same number of edits at random places and at the wrong place
   run beside every real one.
5. **Predictions are sealed first, and misses stay.** Each spec is committed before its program
   runs; each judgement lists what was met and what was missed.
6. **Runs are the operator's.** Nothing here starts, scales or restarts itself.

## Evidence so far, before V45

These were measured before this file was written and are what suggested adopting the law here:

- Judged on the text it was diagnosed from, removing 75,696 entries looked like a 4.8% gain; on
  two independent texts it was 1.4% (V42).
- Removing all 1,437,697 entries that looked harmful on the diagnosis text did worse on
  independent text than removing the worst 7,570 (V42).
- A layer cut to 1/256 guiding a full layer cost 22.8%; a full layer guiding a cut one cost
  nothing extra (V42).

They are consistent with the law. They were not designed as a test of it; V45 is.

## V45, the first deliberate test (November 13, 2026): not shown in the predicted form

Three loops ran six rounds of diagnose-and-remove on the surgery patient
(`results/JUDGE-RECURRENCE729-V45.hbp`).

| Prediction | Result |
|---|---|
| The ungated loop improves in its own view every round | Met |
| The ungated loop gets worse on independent text after round 3 or earlier | **Missed.** It kept improving on both verdict texts at every round |
| The gated loop stops itself before round 6 | **Missed.** The gate text accepted all six proposals |
| The watched loop ends at least as well with fewer cuts | Mixed: 28% of the cuts; better on one verdict text, worse on the other |

What the rows do show:

- **The loop's own view is too good by about a factor of two.** After six rounds it reads −4.65%
  on its diagnosis text; the verdict texts read −2.27% and −1.85%. A diagnosis is still not a verdict.
- **The real gain shrinks fast:** −1.58%, −0.31%, −0.21%, −0.08%, −0.05%, −0.04% per round on the
  first verdict text.
- **The watcher buys efficiency:** about the same result with 12,569 cuts in place of 45,420.

So on this network, six rounds of small cuts did not reach the point where an ungated loop turns
harmful. V42 did show that cutting everything a diagnosis flags does worse than cutting a little,
so the drift exists at larger cuts. The law stays NAMED here: stated, tested once, and not yet
shown by a test designed to show it. A longer loop, or larger cuts per round, is the test that
could.

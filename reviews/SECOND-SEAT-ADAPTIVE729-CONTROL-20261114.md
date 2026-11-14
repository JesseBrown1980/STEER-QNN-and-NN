# Second-seat adversarial control: ordinary adaptive order-2 coder

Related request: [STEER issue #1](https://github.com/JesseBrown1980/STEER-QNN-and-NN/issues/1)
and the V50/V57 referee claims.

**Status: executable instrument tests only until the private stream is supplied.**
Sealed specification: \`specs/SPEC-ADAPTIVE729-SECOND-SEAT-20261114.md\`,
committed before the source. This is not part of the V62 frozen-mixer study.

The central question is whether a **single plain global order-2 predictor**
with exactly the same bit context, probability cost table, online
update rate 1/16, and evaluation text can equal or beat the V57
729-table network's adaptive referee. This control uses no boxes,
no guide, no QNN inner tables, no density layers and no autapses.

Four arms are run on each of four independent cases, using the
original V57 held-out tails and case order:

- \`COLD_STATIC\`: 50:50 all bits.
- \`COLD_ADAPT_R4\`: 50:50 initial values, update the same context
  after scoring every bit with \`p += (target-p)>>4\`.
- \`TRAIN_STATIC\`: corpus-global order-2 bit-tree counts, trained
  on the prefix of every block, held fixed during evaluation.
- \`TRAIN_ADAPT_R4\`: same pretrained global table, with the
  identical online referee update after each bit.

Pretraining skips the last 2,048 bytes of **every** block,
including evaluation blocks; it resets byte context at training
block boundaries. Test-case context persists **across** held-out
tail boundaries and resets only between the four separate cases.
The shuffled test order uses the exact original \`h16\` sorting key.
The scoring function is the original exact
\`pressure_boxes::cost_table\`, with totals in 1/256-bit
and round-to-bytes once per whole text.

All online model arms are capped at **5,800,000 touched contexts**.
If an arm hits that cap it returns \`RESOURCE_HOLD\`
without a valid comparative verdict. A sparse Rust HashMap holds
the online overlay, and a dense \`65536*256\` lookup table stores
global pretrained probabilities. The number of eligible online
positions is matched conservatively to the largest V57 learned
entry count, but the corpus-trained architectures have different
effective capacities and must be characterized separately.

The source row file \`results/RESULT-COMBINED729-V57.hbp\`
provides all four \`BOX_PLAIN,N_STATIC_PLAIN\` and
\`BOX_PLAIN,N_LEARN_PLAIN\` comparisons. No refitting to V57 is
allowed. A four-case win by \`TRAIN_ADAPT_R4\` would be substantial
counterevidence against the idea that the 8% gain requires
729 QNN cells. Failure to win is inconclusive about unique
architecture because the global corpus model is not
capacity-matched to 729 trained table histories.

## Reproduce

For actual stream data, with no file bytes ever committed:

    cargo run --release --bin adaptive_control729 -- /path/to/the/2692693942-byte-stream

For instrument-only synthetic tests:

    cargo test --lib adaptive_control
    cargo run --bin adaptive_control729 -- --self-test

The implementation emits \`ADAPTIVE_CELL\` rows with scored
bytes, exact total bit-cost units, number of online changed keys,
source V57 bytes and case names, plus \`ADAPTIVE_COMPARE\`
and a final \`ADAPTIVE_VERDICT\` count.

The dedicated hosted workflow runs **only instrument tests**
and verifies the original V57 Git blob.
It does **not** have the private 2.69GB stream and cannot
produce experimental outcomes without that input.

## Separate remaining issue-1 attacks

- B: repeat V59 guide/answer asymmetry against a random gate with
  exactly the observed switch rate, matching training exposure.
- C: apply the train-ranked 1/16 information core to held-out
  blocks on which no corresponding table was trained.
- V62: frozen-mixer averaging vs last snapshot; maintain its
  own sealed spec and do not mix results with this control.

This control is designed to **invalidate or narrow claims** if
ordinary online modelling performs just as well.

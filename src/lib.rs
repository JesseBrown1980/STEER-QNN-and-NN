//! STEER: pressure boxes and steered layer stacks on codex-table networks. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

/// Copied unchanged from `JesseBrown1980/GGUF-files-operating-as-trained-quantum-neural-network-models`,
/// `src/bigint.rs`, git blob `23f5447e352e9b8928281f341059d2f25f22bb60`.
pub mod bigint;
pub mod lazy;
pub mod pressure_boxes;
pub mod surgery;

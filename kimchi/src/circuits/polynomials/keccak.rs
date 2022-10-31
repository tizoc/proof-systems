//! Keccak gadget
use ark_ff::PrimeField;

use crate::circuits::gate::CircuitGate;

impl<F: PrimeField> CircuitGate<F> {
    /// Create the Keccak gadget
    /// TODO: right now it only creates a Generic gate followed by the Xor64 gates
    pub fn create_keccak(new_row: usize) -> (usize, Vec<Self>) {
        Self::create_xor(new_row, 64)
    }
}

/// Creates the 5x5 table of rotation bits for Keccak modulo 64
/// | y \ x |  0 |  1 |  2 |  3 |  4 |
/// | ----- | -- | -- | -- | -- | -- |
/// | 0     |  0 |  1 | 62 | 28 | 27 |
/// | 1     | 36 | 44 |  6 | 55 | 20 |
/// | 2     |  3 | 10 | 43 | 25 | 39 |
/// | 3     | 41 | 45 | 15 | 21 |  8 |
/// | 4     | 18 |  2 | 61 | 56 | 14 |
pub const ROT_TAB: [[u32; 5]; 5] = [
    [0, 36, 3, 41, 18],
    [1, 44, 10, 45, 2],
    [62, 6, 43, 15, 61],
    [28, 55, 25, 21, 56],
    [27, 20, 39, 8, 14],
];
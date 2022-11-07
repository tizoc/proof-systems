use std::marker::PhantomData;

use ark_ff::PrimeField;

use crate::circuits::{
    argument::{Argument, ArgumentEnv, ArgumentType},
    expr::constraints::ExprOps,
    gate::GateType,
};

//~ `And16` - Chainable AND constraints for words of multiples of 16 bits.
//~
//~ * This circuit gate is used to constrain that `in1` anded with `in2` equals `out`
//~ * The length of `in1`, `in2` and `out` must be the same and a multiple of 16bits.
//~ * This gate operates on the `Curr` and `Next` rows.
//~
//~ It uses three different types of constraints
//~ * copy          - copy to another cell (32-bits)
//~ * plookup       - and-table plookup (4-bits)
//~ * decomposition - the constraints inside the gate
//~
//~ The 4-bit crumbs are assumed to be laid out with `0` column being the least significant crumb.
//~ Given values `in1`, `in2` and `out`, the layout (identical to that of Xor16) looks like this:
//~
//~ | Column |          `Curr`  |          `Next`  |
//~ | ------ | ---------------- | ---------------- |
//~ |      0 | copy     `in1`   | copy     `in1'`  |
//~ |      1 | copy     `in2`   | copy     `in2'`  |
//~ |      2 | copy     `out`   | copy     `out'`  |
//~ |      3 | plookup0 `in1_0` |                  |
//~ |      4 | plookup1 `in1_1` |                  |
//~ |      5 | plookup2 `in1_2` |                  |
//~ |      6 | plookup3 `in1_3` |                  |
//~ |      7 | plookup0 `in2_0` |                  |
//~ |      8 | plookup1 `in2_1` |                  |
//~ |      9 | plookup2 `in2_2` |                  |
//~ |     10 | plookup3 `in2_3` |                  |
//~ |     11 | plookup0 `out_0` |                  |
//~ |     12 | plookup1 `out_1` |                  |
//~ |     13 | plookup2 `out_2` |                  |
//~ |     14 | plookup3 `out_3` |                  |
//~
//~ One single gate with next values of `in1'`, `in2'` and `out'` being zero can be used to check
//~ that the original `in1`, `in2` and `out` had 16-bits. We can chain this gate 4 times as follows
//~ to obtain a gadget for 64-bit words AND:
//~
//~ | Row | `CircuitGate` | Purpose                                    |
//~ | --- | ------------- | ------------------------------------------ |
//~ |   0 | `And16`       | And 2 least significant bytes of the words |
//~ |   1 | `And16`       | And next 2 bytes of the words              |
//~ |   2 | `And16`       | And next 2 bytes of the words              |
//~ |   3 | `And16`       | And 2 most significant bytes of the words  |
//~ |   4 | `Zero`        | Zero values, can be reused as generic gate |
//~
#[derive(Default)]
pub struct And16<F>(PhantomData<F>);

impl<F> Argument<F> for And16<F>
where
    F: PrimeField,
{
    const ARGUMENT_TYPE: ArgumentType = ArgumentType::Gate(GateType::And16);
    const CONSTRAINTS: u32 = 3;

    // Constraints for And16
    //   * Operates on Curr and Next rows
    //   * Constrain the decomposition of `in1`, `in2` and `out` of multiples of 16 bits
    //   * The actual AND is performed thanks to the plookups of 4-bit ANDs.
    fn constraint_checks<T: ExprOps<F>>(env: &ArgumentEnv<F, T>) -> Vec<T> {
        super::xor::Xor16::constraint_checks(&env)
    }
}

use ark_ff::{FftField, PrimeField};

use circuit_construction::{Cs, Var};

use crate::transcript::{Absorb, VarSponge};
use crate::types::polynomials::ShiftEval;

/// The evaluation of a (possibly chunked) polynomial.
/// The least significant chunk is first.
#[derive(Clone, Copy, Debug)]
pub struct VarEval<F: FftField + PrimeField, const C: usize> {
    pub(crate) chunks: [Var<F>; C],
}

// For exactly one chunk, we can use a polynomial opening as a variable
impl<F> AsRef<Var<F>> for VarEval<F, 1>
where
    F: FftField + PrimeField,
{
    fn as_ref(&self) -> &Var<F> {
        &self.chunks[0]
    }
}

// In general a polynomial opening is a slice of variables
impl<F: FftField + PrimeField, const C: usize> AsRef<[Var<F>]> for VarEval<F, C> {
    fn as_ref(&self) -> &[Var<F>] {
        &self.chunks
    }
}

impl<F> Into<VarEval<F, 1>> for Var<F>
where
    F: FftField + PrimeField,
{
    fn into(self) -> VarEval<F, 1> {
        VarEval { chunks: [self] }
    }
}

impl<F> From<VarEval<F, 1>> for Var<F>
where
    F: FftField + PrimeField,
{
    fn from(open: VarEval<F, 1>) -> Var<F> {
        open.chunks[0]
    }
}

impl<F: FftField + PrimeField, const N: usize> VarEval<F, N> {
    pub fn mul<C: Cs<F>>(&self, cs: &mut C, s: Var<F>) -> Self {
        let mut result = self.clone();
        for i in 0..N {
            result.chunks[i] = cs.mul(result.chunks[i], s);
        }
        result
    }

    pub fn add<C: Cs<F>>(&self, cs: &mut C, other: &Self) -> Self {
        let mut result = self.clone();
        for i in 0..N {
            result.chunks[i] = cs.add(result.chunks[i], other.chunks[i]);
        }
        result
    }

    /// Combine multiple polynomial opening using a random linear combination
    ///
    /// Given openings to polynomials: f_0(X), ..., f_l(X)
    /// yields an opening to the polynomial: g(X) = f_0(X) + r * f_1(X) + ... + r^l * f_l(X)
    ///
    /// Panics if the combination is empty
    pub fn combine<'a, I: Iterator<Item = Self>, C: Cs<F>>(
        cs: &mut C,
        openings: I,
        r: Var<F>,
    ) -> Self {
        // start with most significant term (r^l term)
        let openings = openings.collect::<Vec<_>>();
        let mut openings = openings.into_iter().rev();

        // use Horner's rule
        let mut result = openings.next().expect("empty linear combination").clone();
        for open in openings {
            result = result.mul(cs, r);
            result = result.add(cs, &open);
        }
        result
    }

    /// Combines the evaluation chunks f_0(x), f_1(m), ..., f_m(x) to a single evaluation
    /// f(x) = f_0(x) + x^N f_1(x) + ... + x^{m N} f_m(x)
    ///
    fn collapse<C: Cs<F>>(&self, cs: &mut C, xn: &ShiftEval<F>) -> Var<F> {
        // iterate over coefficients:
        // most-to-least significant
        let mut chk = self.chunks[..].iter().rev().cloned();

        // the initial sum is the most significant term
        let mut sum = chk.next().expect("zero chunks in poly.");

        // shift by pt and add next chunk
        for c in chk {
            sum = cs.mul(sum, xn.as_ref().clone());
            sum = cs.add(sum, c);
        }

        sum
    }
}

impl<F: FftField + PrimeField, const N: usize> Absorb<F> for VarEval<F, N> {
    fn absorb<C: Cs<F>>(&self, cs: &mut C, sponge: &mut VarSponge<F>) {
        sponge.absorb(cs, &self.chunks);
    }
}
use circuit_construction::{Constants, Cs, Var};

use ark_ff::{BigInteger, FftField, FpParameters, PrimeField};

mod sponge;
mod utils;

use super::Context;

pub use sponge::{Absorb, VarSponge};

use std::mem;
use std::ops::{Deref, DerefMut};

use std::marker::PhantomData;

use utils::{decompose, lift, need_decompose, transfer_hash};

struct Public<F: FftField + PrimeField> {
    var: Var<F>,
    size: usize,
}

// A "container type"
struct Side<F: FftField + PrimeField> {
    sponge: VarSponge<F>, // sponge constrained inside the proof system
    merged: bool,         // has the current state been merged with the other side?
}

impl<F: FftField + PrimeField> Side<F> {
    fn new(constants: Constants<F>) -> Self {
        Self {
            sponge: VarSponge::new(constants.clone()),
            merged: true, // sponges start "syncronized":
                          // the empty Fr transcript has been absorbed into the Fp sponge by definition
        }
    }
}

struct Inner<Fp, Fr, CsFp, CsFr>
where
    Fp: FftField + PrimeField,
    Fr: FftField + PrimeField,
{
    fp: Side<Fp>,
    fr: Side<Fr>,
    _ph: PhantomData<(CsFp, CsFr)>,
}

pub struct Arthur<Fp, Fr, CsFp, CsFr>
where
    Fp: FftField + PrimeField,
    Fr: FftField + PrimeField,
{
    inner: Option<Inner<Fp, Fr, CsFp, CsFr>>,
}

/// Describes a type which can be "squeezed" (generated) from the sponge
pub trait Challenge<F: FftField + PrimeField> {
    fn generate<C: Cs<F>>(cs: &mut C, sponge: &mut VarSponge<F>) -> Self;
}

// Can generate a variable from the same field
impl<F: FftField + PrimeField> Challenge<F> for Var<F> {
    fn generate<C: Cs<F>>(cs: &mut C, sponge: &mut VarSponge<F>) -> Self {
        sponge.squeeze(cs)
    }
}

pub struct Msg<T> {
    value: T,
}

impl<T> From<T> for Msg<T> {
    fn from(value: T) -> Self {
        Self { value }
    }
}

pub trait Receivable<F: FftField + PrimeField> {
    type Dst;
    fn unpack<C: Cs<F>>(self, cs: &mut C, sponge: &mut VarSponge<F>) -> Self::Dst;
}

impl<F, H> Receivable<F> for Msg<H>
where
    F: FftField + PrimeField,
    H: Absorb<F>,
{
    type Dst = H;

    fn unpack<C: Cs<F>>(self, cs: &mut C, sponge: &mut VarSponge<F>) -> Self::Dst {
        self.value.absorb(cs, sponge);
        self.value
    }
}

/// Convience: allows receiving a Vec of messages, or vec of vec of messages
/// (you get the point)
impl<F, T> Receivable<F> for Vec<T>
where
    F: FftField + PrimeField,
    T: Receivable<F>,
{
    type Dst = Vec<T::Dst>;

    fn unpack<C: Cs<F>>(self, cs: &mut C, sponge: &mut VarSponge<F>) -> Self::Dst {
        self.into_iter().map(|m| m.unpack(cs, sponge)).collect()
    }
}

impl<Fp, Fr, CsFp, CsFr> Arthur<Fp, Fr, CsFp, CsFr>
where
    Fp: FftField + PrimeField,
    Fr: FftField + PrimeField,
    CsFp: Cs<Fp>,
    CsFr: Cs<Fr>,
{
    pub fn new(ctx: &Context<Fp, Fr, CsFp, CsFr>) -> Self {
        Self {
            inner: Some(Inner::new(ctx)),
        }
    }

    #[must_use]
    pub fn recv<R: Receivable<Fp>>(
        &mut self,
        ctx: &mut Context<Fp, Fr, CsFp, CsFr>,
        msg: R,
    ) -> R::Dst {
        self.inner.as_mut().unwrap().recv(ctx, msg)
    }

    /// Generate a challenge over the current field
    #[must_use]
    pub fn challenge<C: Challenge<Fp>>(&mut self, ctx: &mut Context<Fp, Fr, CsFp, CsFr>) -> C {
        self.inner.as_mut().unwrap().challenge(ctx)
    }

    #[must_use]
    pub fn flip<T, F: FnOnce(&mut Arthur<Fr, Fp, CsFr, CsFp>) -> T>(&mut self, scope: F) -> T {
        // flip the Arthur
        let mut merlin = Arthur {
            inner: self.inner.take().map(|m| m.flipped()),
        };

        // invoke scope
        let msg = scope(&mut merlin);

        // flip back
        self.inner = merlin.inner.map(|m| m.flipped());
        msg
    }

    #[must_use]
    pub fn recv_fr<H: Absorb<Fr>>(
        &mut self,
        ctx: &mut Context<Fp, Fr, CsFp, CsFr>,
        msg: Msg<H>,
    ) -> H {
        ctx.flip(|ctx| {
            // flip the context
            self.flip(|m| {
                // flip the transcript
                m.recv(ctx, msg) // receive
            })
        })
    }

    #[must_use]
    pub fn challenge_fr<C: Challenge<Fr>>(&mut self, ctx: &mut Context<Fp, Fr, CsFp, CsFr>) -> C {
        ctx.flip(|ctx| {
            self.flip(|m| {
                // flip the transcript
                m.challenge(ctx)
            })
        })
    }
}

impl<Fp, Fr, CsFp, CsFr> Inner<Fp, Fr, CsFp, CsFr>
where
    Fp: FftField + PrimeField,
    Fr: FftField + PrimeField,
    CsFp: Cs<Fp>,
    CsFr: Cs<Fr>,
{
    fn new(ctx: &Context<Fp, Fr, CsFp, CsFr>) -> Self {
        Self {
            fp: Side::new(ctx.fp.constants.clone()),
            fr: Side::new(ctx.fr.constants.clone()),
            _ph: PhantomData,
        }
    }

    /// Receive a message from the prover
    #[must_use]
    fn recv_old<H: Absorb<Fp>>(&mut self, ctx: &mut Context<Fp, Fr, CsFp, CsFr>, msg: Msg<H>) -> H {
        self.fp.merged = false; // state updated since last squeeze
        msg.value.absorb(&mut ctx.fp.cs, &mut self.fp.sponge);
        msg.value
    }

    /// Receive a message from the prover
    #[must_use]
    fn recv<R: Receivable<Fp>>(&mut self, ctx: &mut Context<Fp, Fr, CsFp, CsFr>, msg: R) -> R::Dst {
        self.fp.merged = false; // state updated since last squeeze
        msg.unpack(&mut ctx.fp.cs, &mut self.fp.sponge)
    }

    /// Generate a challenge over the current field
    #[must_use]
    fn challenge<C: Challenge<Fp>>(&mut self, ctx: &mut Context<Fp, Fr, CsFp, CsFr>) -> C {
        // check if we need to merge the states
        if !self.fr.merged {
            // merge the "foreign sponge" by adding the current state to the statement
            let st_fr: Var<Fr> = self.fr.sponge.squeeze(&mut ctx.fr.cs);
            let st_fp: Var<Fp> = ctx.fp.cs.var(|| transfer_hash(st_fr.value.unwrap()));

            // absorb commitment to "foreign sponge"
            st_fp.absorb(&mut ctx.fp.cs, &mut self.fp.sponge);
            self.fr.merged = true;
        }

        // squeeze "native sponge" (Fp)
        C::generate(&mut ctx.fp.cs, &mut self.fp.sponge)
    }

    fn flipped(self) -> Inner<Fr, Fp, CsFr, CsFp> {
        Inner {
            fp: self.fr,
            fr: self.fp,
            _ph: PhantomData,
        }
    }
}

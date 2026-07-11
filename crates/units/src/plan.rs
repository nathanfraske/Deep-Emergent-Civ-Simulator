// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The Tier-2 load-time scale planner (R-UNITS-PIN slice 2). A law is expressed as a [`LawExpr`], a typed op
//! graph over scaled quantities. The node kinds are the FIXED arithmetic floor (the operations the number
//! system supports, the same status as the physics kernels being fixed Rust); the LAWS (compositions of them)
//! and the quantities' envelopes and floors are open DATA (Principle 11).
//!
//! [`plan`] walks a `LawExpr` once at load and serves all three roles the design converges here: it sizes each
//! node's intermediate WIDTH from its measured exponent interval (i128 for a single op, i256 for a chained law
//! whose accumulator exceeds 127 bits, never i128 by default), assigns each node a SCALE by
//! [`crate::derive_scale_bits`] on its envelope, and ENFORCES the floor invariant, failing loud at load when a
//! `Div` or `Ln` node's operand carries neither a declared physical floor nor a declared physical-limit-at-zero
//! (the observer-independence guard, Principle 10: a bound that rode the storage epsilon becomes a physics
//! artifact once Tier 2 lowers that epsilon). The result is a fixed per-node plan, so the per-tick evaluation
//! is a fixed deterministic integer function.

use crate::tier2::WideAccum;
use crate::{derive_scale_bits, DerivedScale};

/// A quantity's declared magnitude envelope: the floor-base-2 logarithms of its largest and smallest bound
/// magnitudes, and, if it carries one, the log2 of its declared PHYSICAL FLOOR (below which it is treated as
/// zero or absent). The envelope and the floor are per-world data (reserved-with-basis); the planner reads
/// them, it authors none.
#[derive(Clone, Copy, Debug)]
pub struct QuantityEnvelope {
    pub lo_log2: i32,
    pub hi_log2: i32,
    /// `Some(log2)` when the quantity declares a physical floor; `None` when it does not (a `Div`/`Ln` on it
    /// then fails loud unless the node declares a physical-limit-at-zero).
    pub physical_floor_log2: Option<i32>,
}

/// The zero-boundary declaration a `Div` or `Ln` node carries: either its operand must resolve a declared
/// physical floor, or the node declares that its zero-boundary is an intentional physical limit (a point-load
/// pressure cap, a Nernst depleted-activity zero), which needs no floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZeroBoundary {
    /// The operand (denominator, log argument) must carry a declared `physical_floor_log2`; fail loud otherwise.
    RequiresFloor,
    /// The node's zero-boundary is a deliberate physical limit; no operand floor is required.
    PhysicalLimitAtZero,
}

/// The op graph a law is expressed as. The node set is the fixed computation floor; the tree (the law) is data.
#[derive(Clone, Debug)]
pub enum LawExpr {
    /// An input quantity, resolved to its [`QuantityEnvelope`].
    Input(u32),
    /// A rational law-constant, carrying the floor-base-2 log of its magnitude (the one authored place, P9).
    Const {
        log2: i32,
    },
    Mul(Box<LawExpr>, Box<LawExpr>),
    Div {
        num: Box<LawExpr>,
        den: Box<LawExpr>,
        zero: ZeroBoundary,
    },
    Add(Box<LawExpr>, Box<LawExpr>),
    Sub(Box<LawExpr>, Box<LawExpr>),
    /// An integer power, the chain the wide accumulator carries.
    Powi(Box<LawExpr>, u32),
    /// A square root; the quarter-power is two of these.
    Isqrt(Box<LawExpr>),
    Ln {
        arg: Box<LawExpr>,
        zero: ZeroBoundary,
    },
}

/// The intermediate integer width a node's un-rounded accumulator needs, sized per-node by measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Width {
    /// The accumulator fits a 127-bit signed `i128` (a single op, or a small chain).
    I128,
    /// The accumulator needs the eight-sub-limb `i256` (a chained law, the flagship `sigma * T^4` at ~210 bits).
    I256,
    /// The accumulator exceeds 255 bits; the planner surfaces this rather than silently truncate, so the owner
    /// widens the number system or the law is re-scaled.
    Wider,
}

/// A node's derived plan: its exponent interval, its scale, the intermediate width, and the significance
/// window flag.
#[derive(Clone, Copy, Debug)]
pub struct PlannedNode {
    pub lo_log2: i32,
    pub hi_log2: i32,
    pub scale_bits: u32,
    /// The magnitude bits the node's un-rounded wide accumulator holds (before the single terminal round).
    pub wide_bits: u32,
    pub width: Width,
    pub windowed: bool,
}

/// The canonical fixed-point scale, Q32.32, the scale a node keeps when its envelope already fits it.
const CANONICAL_SCALE: u32 = 32;

fn width_of(wide_bits: u32) -> Width {
    if wide_bits <= 127 {
        Width::I128
    } else if wide_bits <= 255 {
        Width::I256
    } else {
        Width::Wider
    }
}

/// Plan a law's op graph at load: derive every node's scale, size its intermediate width, and enforce the
/// floor invariant. `resolve` reads an input quantity's declared envelope; `sig_target` and `guard` are the
/// global reserved significance and headroom the scale derivation reads. Returns the ROOT node's plan, or an
/// error naming the first `Div`/`Ln` node whose operand resolves neither a declared floor nor a declared
/// physical-limit-at-zero (fail loud, the same discipline as an unset reserved value).
pub fn plan(
    expr: &LawExpr,
    resolve: &dyn Fn(u32) -> QuantityEnvelope,
    sig_target: u32,
    guard: u32,
) -> Result<PlannedNode, String> {
    match expr {
        LawExpr::Input(q) => {
            let env = resolve(*q);
            Ok(leaf(env.lo_log2, env.hi_log2, sig_target, guard))
        }
        LawExpr::Const { log2 } => Ok(leaf(*log2, *log2, sig_target, guard)),
        LawExpr::Mul(a, b) => {
            let (pa, pb) = (
                plan(a, resolve, sig_target, guard)?,
                plan(b, resolve, sig_target, guard)?,
            );
            Ok(node(
                pa.lo_log2 + pb.lo_log2,
                pa.hi_log2 + pb.hi_log2,
                pa.wide_bits + pb.wide_bits,
                sig_target,
                guard,
            ))
        }
        LawExpr::Div { num, den, zero } => {
            let pn = plan(num, resolve, sig_target, guard)?;
            let pd = plan(den, resolve, sig_target, guard)?;
            // The denominator's low bound must come from a declared floor, not the storage epsilon.
            let den_lo = resolve_floor_lo(den, resolve, *zero, "Div denominator")?;
            // value interval a/b: [lo_a - hi_b, hi_a - den_lo]; the divide's wide intermediate shifts the
            // numerator up by the scale span, so its bits are the numerator's plus that span.
            let lo = pn.lo_log2 - pd.hi_log2;
            let hi = pn.hi_log2 - den_lo;
            let shift = (pd.scale_bits as i32 + node_scale(lo, hi, sig_target, guard) as i32
                - pn.scale_bits as i32)
                .max(0) as u32;
            Ok(node(lo, hi, pn.wide_bits + shift, sig_target, guard))
        }
        LawExpr::Add(a, b) | LawExpr::Sub(a, b) => {
            let (pa, pb) = (
                plan(a, resolve, sig_target, guard)?,
                plan(b, resolve, sig_target, guard)?,
            );
            // The exact sum aligns to the finer scale; its magnitude is the larger operand's plus a carry bit.
            let lo = pa.lo_log2.min(pb.lo_log2);
            let hi = pa.hi_log2.max(pb.hi_log2) + 1;
            Ok(node(
                lo,
                hi,
                pa.wide_bits.max(pb.wide_bits) + 1,
                sig_target,
                guard,
            ))
        }
        LawExpr::Powi(a, n) => {
            let pa = plan(a, resolve, sig_target, guard)?;
            let exp = *n as i32;
            Ok(node(
                pa.lo_log2 * exp,
                pa.hi_log2 * exp,
                pa.wide_bits.saturating_mul(*n),
                sig_target,
                guard,
            ))
        }
        LawExpr::Isqrt(a) => {
            let pa = plan(a, resolve, sig_target, guard)?;
            Ok(node(
                pa.lo_log2.div_euclid(2),
                (pa.hi_log2 + 1).div_euclid(2),
                pa.wide_bits.div_ceil(2) + 1,
                sig_target,
                guard,
            ))
        }
        LawExpr::Ln { arg, zero } => {
            let pa = plan(arg, resolve, sig_target, guard)?;
            resolve_floor_lo(arg, resolve, *zero, "Ln argument")?;
            // ln maps a positive value to roughly [-a few, a few]; a small bounded interval.
            Ok(node(-8, 8, pa.wide_bits, sig_target, guard))
        }
    }
}

/// Evaluate a `LawExpr` to a single scaled mantissa at `output_scale`, computing the WHOLE chain in one
/// [`WideAccum`] and rounding ONCE at the end (the single-round-per-chain discipline the slice-1 measurement
/// showed exact to the arbitrary-precision oracle). `input` resolves each `Input(q)` to its `(bits, scale)`
/// value. Returns `None` on a wide-accumulator overflow, an `Add`/`Sub` whose two sub-chains land at
/// different running scales, or a node kind this evaluator does not yet handle.
///
/// The supported shape is the multiplicative chain with a difference-of-powers core that the flagship radiant
/// law `sigma * (T_hot^4 - T_cold^4) * emissivity * area` needs: `Input`, `Powi`, `Add`/`Sub` of equal-scale
/// chains, and `Mul` where at least ONE operand is an `Input` leaf that folds into the other side's chain as a
/// scalar (the wide accumulator multiplies a scalar mantissa in, not another wide chain). A `Mul` of two
/// non-leaf chains, and the `Const`/`Div`/`Ln`/`Isqrt` node kinds inside a chain, return `None` here: each is
/// added when a lifted law first needs it (its own slice extension), rather than authored speculatively.
pub fn evaluate(
    expr: &LawExpr,
    input: &dyn Fn(u32) -> (i64, u32),
    output_scale: u32,
) -> Option<i64> {
    eval_accum(expr, input)?.round_to_scale(output_scale)
}

/// Whether a node is an `Input` leaf whose `(bits, scale)` value folds into a chain as a scalar multiply.
fn leaf_value(expr: &LawExpr, input: &dyn Fn(u32) -> (i64, u32)) -> Option<(i64, u32)> {
    match expr {
        LawExpr::Input(q) => Some(input(*q)),
        _ => None,
    }
}

fn eval_accum(expr: &LawExpr, input: &dyn Fn(u32) -> (i64, u32)) -> Option<WideAccum> {
    match expr {
        LawExpr::Input(q) => {
            let (bits, scale) = input(*q);
            Some(WideAccum::new(bits, scale))
        }
        LawExpr::Powi(a, n) => {
            let (bits, scale) = leaf_value(a, input)?;
            WideAccum::power(bits, scale, *n)
        }
        LawExpr::Mul(a, b) => {
            // Fold whichever side is a scalar `Input` leaf into the other side's chain. The wide accumulator
            // multiplies a scalar mantissa in, so a Mul of two non-leaf chains is unsupported here (None).
            if let Some((bits, scale)) = leaf_value(b, input) {
                eval_accum(a, input)?.mul(bits, scale)
            } else if let Some((bits, scale)) = leaf_value(a, input) {
                eval_accum(b, input)?.mul(bits, scale)
            } else {
                None
            }
        }
        LawExpr::Add(a, b) => eval_accum(a, input)?.add(&eval_accum(b, input)?),
        LawExpr::Sub(a, b) => eval_accum(a, input)?.sub(&eval_accum(b, input)?),
        // Const (needs a value binding), Div, Ln, and Isqrt inside a chain are added per lifted law.
        _ => None,
    }
}

/// Confirm a `Div`/`Ln` operand resolves its zero-boundary, and return its low log2 bound (the floor). Fails
/// loud when the operand declares neither a physical floor nor a physical-limit-at-zero on the node.
fn resolve_floor_lo(
    operand: &LawExpr,
    resolve: &dyn Fn(u32) -> QuantityEnvelope,
    zero: ZeroBoundary,
    role: &str,
) -> Result<i32, String> {
    match zero {
        ZeroBoundary::PhysicalLimitAtZero => {
            // A declared intentional limit needs no operand floor; the operand's own low bound stands.
            Ok(operand_lo(operand, resolve))
        }
        ZeroBoundary::RequiresFloor => match operand {
            LawExpr::Input(q) => match resolve(*q).physical_floor_log2 {
                Some(floor) => Ok(floor),
                None => Err(format!(
                    "floor invariant: the {role} (quantity {q}) has no declared physical_floor and the node \
                     declares no physical_limit_at_zero; it would ride the storage epsilon (P10). Declare one."
                )),
            },
            _ => Err(format!(
                "floor invariant: the {role} is a composed expression that can reach zero (a difference-divisor) \
                 and the node declares no physical_limit_at_zero; declare a floor or a limit."
            )),
        },
    }
}

fn operand_lo(operand: &LawExpr, resolve: &dyn Fn(u32) -> QuantityEnvelope) -> i32 {
    match operand {
        LawExpr::Input(q) => resolve(*q).lo_log2,
        LawExpr::Const { log2 } => *log2,
        _ => i32::MIN / 2, // a composed operand's low bound is unknown here; the node's limit declaration covers it
    }
}

fn leaf(lo: i32, hi: i32, sig_target: u32, guard: u32) -> PlannedNode {
    // A leaf's wide bits are its mantissa's top bit at its own scale.
    let DerivedScale {
        scale_bits,
        windowed,
    } = derive_scale_bits(hi, lo, sig_target, guard, CANONICAL_SCALE);
    let wide_bits = (hi + scale_bits as i32).max(1) as u32;
    PlannedNode {
        lo_log2: lo,
        hi_log2: hi,
        scale_bits,
        wide_bits,
        width: width_of(wide_bits),
        windowed,
    }
}

fn node(lo: i32, hi: i32, wide_bits: u32, sig_target: u32, guard: u32) -> PlannedNode {
    let DerivedScale {
        scale_bits,
        windowed,
    } = derive_scale_bits(hi, lo, sig_target, guard, CANONICAL_SCALE);
    PlannedNode {
        lo_log2: lo,
        hi_log2: hi,
        scale_bits,
        wide_bits,
        width: width_of(wide_bits),
        windowed,
    }
}

fn node_scale(lo: i32, hi: i32, sig_target: u32, guard: u32) -> u32 {
    derive_scale_bits(hi, lo, sig_target, guard, CANONICAL_SCALE).scale_bits
}

#[cfg(test)]
mod tests {
    use super::*;

    // A fixture resolver for the radiant law's inputs, mirroring the measured envelopes.
    fn radiant_env(q: u32) -> QuantityEnvelope {
        match q {
            0 => QuantityEnvelope {
                lo_log2: -25,
                hi_log2: -24,
                physical_floor_log2: None,
            }, // sigma
            1 => QuantityEnvelope {
                lo_log2: 7,
                hi_log2: 9,
                physical_floor_log2: Some(0),
            }, // T (K), floored
            2 => QuantityEnvelope {
                lo_log2: -1,
                hi_log2: 0,
                physical_floor_log2: None,
            }, // emissivity [0,1]
            3 => QuantityEnvelope {
                lo_log2: -4,
                hi_log2: 2,
                physical_floor_log2: Some(-10),
            }, // area, floored
            _ => QuantityEnvelope {
                lo_log2: 0,
                hi_log2: 0,
                physical_floor_log2: None,
            },
        }
    }

    fn boxed(e: LawExpr) -> Box<LawExpr> {
        Box::new(e)
    }

    #[test]
    fn the_flagship_quartic_node_is_sized_i256() {
        // T^4 (T ~ 2^9 at a fine scale) reaches past 127 bits, so the chain node plans to i256.
        let t4 = LawExpr::Powi(boxed(LawExpr::Input(1)), 4);
        let chain = LawExpr::Mul(boxed(LawExpr::Input(0)), boxed(t4));
        let p = plan(&chain, &radiant_env, 30, 1).unwrap();
        assert_eq!(
            p.width,
            Width::I256,
            "the sigma*T^4 chain needs i256 (wide_bits {})",
            p.wide_bits
        );
        assert!(p.wide_bits > 127 && p.wide_bits <= 255);
    }

    #[test]
    fn a_single_op_stays_i128() {
        // emissivity * area, both O(1)-range, fits i128.
        let m = LawExpr::Mul(boxed(LawExpr::Input(2)), boxed(LawExpr::Input(3)));
        let p = plan(&m, &radiant_env, 30, 1).unwrap();
        assert_eq!(p.width, Width::I128);
    }

    #[test]
    fn a_divide_by_a_floored_quantity_plans_but_an_unfloored_one_fails_loud() {
        // T carries a declared floor (quantity 1): a divide by it plans.
        let ok = LawExpr::Div {
            num: boxed(LawExpr::Input(2)),
            den: boxed(LawExpr::Input(1)),
            zero: ZeroBoundary::RequiresFloor,
        };
        assert!(plan(&ok, &radiant_env, 30, 1).is_ok());
        // sigma (quantity 0) has NO declared floor: a RequiresFloor divide by it fails loud.
        let bad = LawExpr::Div {
            num: boxed(LawExpr::Input(2)),
            den: boxed(LawExpr::Input(0)),
            zero: ZeroBoundary::RequiresFloor,
        };
        let err = plan(&bad, &radiant_env, 30, 1).unwrap_err();
        assert!(err.contains("floor invariant"), "unexpected: {err}");
        // But if the node declares a physical limit at zero, it plans without a floor.
        let limit = LawExpr::Div {
            num: boxed(LawExpr::Input(2)),
            den: boxed(LawExpr::Input(0)),
            zero: ZeroBoundary::PhysicalLimitAtZero,
        };
        assert!(plan(&limit, &radiant_env, 30, 1).is_ok());
    }

    #[test]
    fn a_difference_divisor_without_a_declared_limit_fails_loud() {
        // ONE / (inv(a) + inv(b) - inv(c)) as a divide by a composed (Sub) expression that can reach zero:
        // with RequiresFloor and a composed denominator, no per-quantity floor covers it, so it fails loud.
        let recip = LawExpr::Sub(
            boxed(LawExpr::Div {
                num: boxed(LawExpr::Const { log2: 0 }),
                den: boxed(LawExpr::Input(1)),
                zero: ZeroBoundary::RequiresFloor,
            }),
            boxed(LawExpr::Div {
                num: boxed(LawExpr::Const { log2: 0 }),
                den: boxed(LawExpr::Input(3)),
                zero: ZeroBoundary::RequiresFloor,
            }),
        );
        let expr = LawExpr::Div {
            num: boxed(LawExpr::Const { log2: 0 }),
            den: boxed(recip),
            zero: ZeroBoundary::RequiresFloor,
        };
        let err = plan(&expr, &radiant_env, 30, 1).unwrap_err();
        assert!(
            err.contains("difference-divisor") || err.contains("floor invariant"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn scales_derive_from_the_declared_envelope() {
        // sigma at [-25,-24] with sig_target 30 derives the fine scale (~55), not the canonical 32.
        let p = plan(&LawExpr::Input(0), &radiant_env, 30, 1).unwrap();
        assert!(
            p.scale_bits > CANONICAL_SCALE,
            "sigma should derive a finer scale, got {}",
            p.scale_bits
        );
    }

    // ---- slice 4a: the evaluator over the flagship radiant law, byte-neutral (no run-path wire) ----

    use crate::bignum::{BigRat, BigUint};
    use civsim_core::Fixed;

    /// The exact rational value of a scaled mantissa `bits / 2^scale`.
    fn scaled_rat(bits: i64, scale: u32) -> BigRat {
        BigRat::new(
            bits < 0,
            BigUint::from_u64(bits.unsigned_abs()),
            BigUint::from_u64(2).pow(scale),
        )
    }

    /// The flagship radiant law `sigma * (T_hot^4 - T_cold^4) * emissivity * area` as a left-leaning `LawExpr`
    /// where every `Mul` folds one scalar `Input` leaf (sigma, emissivity, area) into the difference-of-quartics
    /// chain. Input ids: 0 sigma, 1 T_hot, 2 T_cold, 3 emissivity, 4 area.
    fn radiant_expr() -> LawExpr {
        let diff = LawExpr::Sub(
            boxed(LawExpr::Powi(boxed(LawExpr::Input(1)), 4)),
            boxed(LawExpr::Powi(boxed(LawExpr::Input(2)), 4)),
        );
        LawExpr::Mul(
            boxed(LawExpr::Mul(
                boxed(LawExpr::Mul(boxed(diff), boxed(LawExpr::Input(0)))),
                boxed(LawExpr::Input(3)),
            )),
            boxed(LawExpr::Input(4)),
        )
    }

    /// The exact rational the radiant law evaluates to for the given scaled inputs.
    fn radiant_oracle(vals: &[(i64, u32); 5]) -> BigRat {
        let t_hot = scaled_rat(vals[1].0, vals[1].1);
        let t_cold = scaled_rat(vals[2].0, vals[2].1);
        let hot4 = t_hot.mul(&t_hot).mul(&t_hot).mul(&t_hot);
        let cold4 = t_cold.mul(&t_cold).mul(&t_cold).mul(&t_cold);
        hot4.sub(&cold4)
            .mul(&scaled_rat(vals[0].0, vals[0].1))
            .mul(&scaled_rat(vals[3].0, vals[3].1))
            .mul(&scaled_rat(vals[4].0, vals[4].1))
    }

    #[test]
    fn the_radiant_law_evaluates_exactly_to_the_bigrat_oracle() {
        // sigma at its fine scale 55, temperatures/emissivity/area at Q32.32. A forge (T_hot ~ 1200 K) losing
        // heat to a 300 K room. Every input a scalar leaf but the difference-of-quartics chain.
        let sigma_fine = BigRat::from_decimal_str("0.00000005670374419")
            .unwrap()
            .round_to_scale(55)
            .unwrap() as i64;
        // The scales here are what the slice-2 planner would derive to keep the single-round chain inside the
        // i256 accumulator (sigma at its fine 55, the temperatures at 24 so the quartic does not overrun 255
        // bits); a raw all-Q32.32 chain overflows, which is why the planner sizes per law.
        let vals: [(i64, u32); 5] = [
            (sigma_fine, 55),
            (1200i64 << 24, 24),                      // T_hot 1200 K
            (300i64 << 24, 24),                       // T_cold 300 K
            (Fixed::from_ratio(9, 10).to_bits(), 32), // emissivity 0.9
            (Fixed::from_ratio(1, 20).to_bits(), 32), // area 0.05 m^2
        ];
        let out_scale = 32u32; // deliver the flux back at Q32.32 for the drain
        let got = evaluate(&radiant_expr(), &|q| vals[q as usize], out_scale)
            .expect("the radiant chain fits the wide accumulator");
        let want = radiant_oracle(&vals).round_to_scale(out_scale).unwrap() as i64;
        assert_eq!(
            got, want,
            "the single-round chain is exact to the rational oracle"
        );
    }

    #[test]
    fn sigma_at_a_fine_scale_reaches_the_result_where_the_q32_truncation_does_not() {
        // The lift's whole point: sigma's precision survives into the flux. Evaluate the law twice, once with
        // sigma at its fine scale (its full ~31-bit mantissa) and once with sigma truncated to Q32.32 (244 raw,
        // the ~8-bit value the hand-written law carries today), and show the two results DIFFER at the output,
        // each matching its OWN sigma's exact oracle. So the finer sigma is not lost in the arithmetic.
        let sigma_fine = BigRat::from_decimal_str("0.00000005670374419")
            .unwrap()
            .round_to_scale(55)
            .unwrap() as i64;
        let base = |sigma: (i64, u32)| -> [(i64, u32); 5] {
            [
                sigma,
                (1200i64 << 24, 24),
                (300i64 << 24, 24),
                (Fixed::ONE.to_bits(), 32), // emissivity 1 (isolate sigma)
                (Fixed::ONE.to_bits(), 32), // area 1
            ]
        };
        let out_scale = 32u32;
        let fine_vals = base((sigma_fine, 55));
        let trunc_vals = base((244, 32)); // sigma as Q32.32, the current truncation
        let fine = evaluate(&radiant_expr(), &|q| fine_vals[q as usize], out_scale).unwrap();
        let trunc = evaluate(&radiant_expr(), &|q| trunc_vals[q as usize], out_scale).unwrap();
        assert_ne!(
            fine, trunc,
            "the fine sigma and the Q32.32 truncation give different flux, so sigma's precision reaches the result"
        );
        // Each matches its own sigma's exact oracle (the evaluator is exact for both).
        assert_eq!(
            fine,
            radiant_oracle(&fine_vals)
                .round_to_scale(out_scale)
                .unwrap() as i64
        );
        assert_eq!(
            trunc,
            radiant_oracle(&trunc_vals)
                .round_to_scale(out_scale)
                .unwrap() as i64
        );
    }
}

//! The damage-insult substrate (R-AGING (c), slices 1 and 2): the data-defined set of ways a body
//! part's own material accrues cumulative DAMAGE ENERGY, its restoring counter (a tissue-turnover
//! REPAIR flux funded by a maintenance-energy draw), and the pure math that turns net accumulated
//! damage into the part's integrity.
//!
//! Operational lifespan is the first-passage time of a per-part damage accumulator against each
//! part's own material tolerance (a body dies when a vital part's accumulated damage reaches its
//! failure energy). This module is the accumulator's KERNELS and its integrity math, landed pure: it
//! reads a tissue material and a situation and returns an energy increment, and it turns an
//! accumulated energy into an integrity in `[0, 1]`. It stores nothing and is wired into no canonical
//! state here; the per-part `damage_energy` accumulator, the per-tick body update, and the
//! first-passage death path are a later slice (the one stated hash change), so this module is
//! byte-neutral against the four `run_world` pins.
//!
//! The discipline (the reframed (c) target): each insult stands on its OWN independently-grounded
//! floor physics, commensurated to a common ENERGY currency (the kilojoule scale the floor's delivered
//! energy and `fracture_energy * crack_area` share) through the floor's own laws, so the ENERGY BRIDGE
//! carries NO free per-insult weight (the wear increment is `V * specific_cut_energy * C_VOL`, all
//! material and floor terms) and no term is justified by the aging outcome it produces. Whatever
//! size-longevity relation emerges from the arithmetic is the output, never a written law. The KERNEL
//! SET is fixed Rust; a genuinely new KIND of insult (a new grounded energy bridge) is a bounded code
//! addition, and which insults a world's beings run, and in what order, is DATA ([`InsultRegistry`]).
//! The mechanism is code, the membership is data (Principle 11).
//!
//! One honest limit in slice 1, flagged not hidden: the Archard wear COEFFICIENT (the tribological-pair
//! property that scales the worn VOLUME upstream in `laws::wear`, not the energy bridge) is carried
//! here as a world-level reserved parameter ([`InsultCaps`]), so the wear insult differentiates across
//! tissues today only through their hardness and specific cutting energy, not through the coefficient.
//! The tool-wear path already sources this coefficient per-material from the material's own
//! `mat.wear_coefficient` axis (`runner.rs`), and the wear insult should do the same when it wires: the
//! re-pin slice reads `mat.wear_coefficient` per tissue, which needs a `wear_coefficient` field on
//! `TissueMaterial` (Agent A's `body.rs` surface, to raise to the gate). Until then the coefficient is
//! a reserved parameter, not yet keyed on the being's own material.
//!
//! Naming: these `Insult*` types are the CONTINUOUS background damage-energy accrual (an aging insult),
//! distinct from `crate::body::Insult` and `apply_insult`, which model a DISCRETE strike-driven wound.
//!
//! Slice 1 admits exactly one kernel, [`InsultKind::Wear`], the one insult whose energy bridge is an
//! exact identity from the floor's own cut model (`civsim_physics::laws::wear_energy`). The thermal,
//! chemical, and metabolic/oxidative insults each need their own grounded bridge or substrate before
//! they can join, and each is a bounded future kernel, not a hardcoded row: the fed-state oxidative
//! insult in particular is a held owner-call (it decides whether a rate-of-living-shaped slope
//! EMERGES from real per-being metabolic and antioxidant data), a row the world selects once the
//! kernel and the owner's ruling land.

use civsim_core::Fixed;
use civsim_physics::laws;

use crate::body::TissueMaterial;

/// A declared insult's id: its slot in a world's [`InsultRegistry`], in registry order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct InsultId(pub u16);

/// The closed set of insult KERNELS: the physically-distinct ways a part's own material accrues
/// damage energy. Fixed Rust, one variant per kernel, each with an independently-grounded, no-free-
/// weight bridge to the common kilojoule energy scale. Which kernels a world's beings SELECT, and in
/// what order, is data ([`InsultRegistry::from_names`]); a genuinely new kind of insult is a bounded
/// code addition here (its own grounded energy bridge), never a data name pointed at a missing law.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InsultKind {
    /// Archard abrasive material loss, commensurated to energy by the cut model's own identity
    /// (`V * specific_cut_energy * C_VOL`, the inverse of `cut_penetrate`), keyed on the tissue's own
    /// hardness and specific cutting energy.
    Wear,
}

impl InsultKind {
    /// The full closed kernel set in canonical order, so [`InsultKind::from_name`] derives from
    /// [`InsultKind::name`] over it (the two cannot diverge) and a round-trip test covers every
    /// kernel. A new variant is added here and, because [`InsultKind::name`] matches exhaustively,
    /// gets a name at the same time.
    pub const ALL: [InsultKind; 1] = [InsultKind::Wear];

    /// This kernel's stable string name, the id world data resolves against. A resolution key only,
    /// consumed at registry build and never stored on the resulting [`InsultDef`].
    pub fn name(self) -> &'static str {
        match self {
            InsultKind::Wear => "wear",
        }
    }

    /// Resolve a kernel from its stable name, the inverse of [`InsultKind::name`], derived over
    /// [`InsultKind::ALL`] so the two cannot disagree. `None` for an unknown name, so the registry
    /// fails loud.
    pub fn from_name(name: &str) -> Option<InsultKind> {
        InsultKind::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// The energy increment (kilojoule scale) this insult delivers into the part's fracture process
    /// this tick, from the part's own tissue MATERIAL and the SITUATION. Every branch stands on an
    /// independently-grounded floor law and commensurates dimensionally through the floor's own
    /// energy bridge, so there is no per-insult weight to tune. `energy_max` is the representability
    /// cap the floor bridge routes an overflow to.
    pub fn energy(
        self,
        mat: &TissueMaterial,
        drive: &InsultDrive,
        caps: &InsultCaps,
        energy_max: Fixed,
    ) -> Fixed {
        match self {
            InsultKind::Wear => laws::wear_energy(
                caps.wear_coefficient_scaled,
                caps.coefficient_scale,
                drive.force,
                drive.distance,
                mat.hardness,
                mat.specific_cut_energy,
                caps.wear_max,
                energy_max,
            ),
        }
    }
}

/// The per-tick SITUATION an insult reads: the mechanical and thermal drives acting on a part. Slice
/// 1 takes it as a parameter (pure); the later re-pin slice reads it from the part's geometry and
/// activity. Only the fields a live kernel reads are load-bearing; the rest are carried for the
/// flagged future kernels so the dispatch keeps one signature.
#[derive(Clone, Copy, Debug, Default)]
pub struct InsultDrive {
    /// Mechanical load on the part (newtons), the Archard wear force.
    pub force: Fixed,
    /// Slide distance over the tick (metres), the Archard wear distance.
    pub distance: Fixed,
}

/// The reserved caps and coefficients the floor insult laws take as config, surfaced not fabricated
/// (CLAUDE.md section 7). Slice 1 passes them in; the re-pin slice sources them from the insult
/// calibration manifest with each value's basis. The Archard coefficient and the caps are the floor's
/// declared ceilings, not owner-realism values.
#[derive(Clone, Copy, Debug)]
pub struct InsultCaps {
    /// The Archard wear coefficient, carried at its own scale. A world-level reserved parameter in
    /// slice 1, NOT yet keyed on the being's own tissue; the target is to source it per-tissue from the
    /// material's own `mat.wear_coefficient` axis when this wires (the `runner.rs` tool-wear precedent),
    /// which needs a `wear_coefficient` field on `TissueMaterial` (Agent A's surface). Basis: real
    /// tribological wear coefficients per material pair.
    pub wear_coefficient_scaled: Fixed,
    /// The scale the wear coefficient is carried at, divided back out in `laws::wear`.
    pub coefficient_scale: Fixed,
    /// The representability ceiling on a per-tick worn volume.
    pub wear_max: Fixed,
}

/// One declared insult in a world's registry: its id (position) and the kernel it runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InsultDef {
    pub id: InsultId,
    pub kind: InsultKind,
}

/// The set of insults a world runs, data-defined and extensible: the kernels are fixed Rust, the
/// membership (which kernels, in what order) is data. Empty by default, so a world that declares no
/// insults accrues no damage (byte-neutral). Sibling of the tissue, affordance-percept, and capability
/// registries.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct InsultRegistry {
    insults: Vec<InsultDef>,
}

impl InsultRegistry {
    /// An empty registry (no insults declared).
    pub fn empty() -> InsultRegistry {
        InsultRegistry {
            insults: Vec::new(),
        }
    }

    /// Build a registry from an ordered list of kernels, ids assigned by position.
    pub fn from_kinds(kinds: &[InsultKind]) -> InsultRegistry {
        InsultRegistry {
            insults: kinds
                .iter()
                .enumerate()
                .map(|(i, &kind)| InsultDef {
                    id: InsultId(i as u16),
                    kind,
                })
                .collect(),
        }
    }

    /// Build a registry from data names, FAILING LOUD on an unknown name (naming the offender) rather
    /// than silently dropping it, and leaking no partial registry. Delegates to [`from_kinds`] once
    /// every name resolves, so the two paths produce byte-identical registries. `from_kinds` is
    /// [`InsultRegistry::from_kinds`].
    pub fn from_names(names: &[&str]) -> Result<InsultRegistry, String> {
        let mut kinds = Vec::with_capacity(names.len());
        for &name in names {
            match InsultKind::from_name(name) {
                Some(kind) => kinds.push(kind),
                None => {
                    return Err(format!(
                        "unknown insult kernel name {name:?}; the kernels are fixed Rust and a new \
                         one is a code change with its own grounded energy bridge, not a data name"
                    ))
                }
            }
        }
        Ok(InsultRegistry::from_kinds(&kinds))
    }

    /// The declared insults, in registry order.
    pub fn defs(&self) -> &[InsultDef] {
        &self.insults
    }

    pub fn len(&self) -> usize {
        self.insults.len()
    }

    pub fn is_empty(&self) -> bool {
        self.insults.is_empty()
    }
}

/// Integrity in `[0, 1]` from a part's accumulated damage energy against its own FAILURE-energy
/// tolerance: `1 - clamp(accumulated / tolerance, 0, 1)`. The math is a generic energy ratio with no
/// mechanical assumption baked in: for a tissue the tolerance is the fracture energy the floor's
/// `fracture_onset` measures delivered energy against (`fracture_energy * crack_area`, a kilojoule
/// energy), and for an alien whose vital-part failure is non-mechanical (field-depletion, thermal,
/// chemical) it is whatever failure-energy that being's own material declares, read from the part's
/// data (crux 4, the per-material death tolerance), so the alien is a data row. Both operands are the
/// same energy scale, so the ratio is dimensionless. A zero-or-negative tolerance is a part with no
/// failure reserve, so already failed (integrity 0), mirroring `laws::wear`'s zero-hardness
/// convention. An unrepresentably large ratio (a tiny tolerance under a large accumulation) saturates
/// to a fully-failed part (`checked_div` returns `None`, read as full damage) rather than wrapping.
pub fn derive_integrity(
    accumulated_damage_energy: Fixed,
    failure_energy_tolerance: Fixed,
) -> Fixed {
    if failure_energy_tolerance.to_bits() <= 0 {
        return Fixed::ZERO;
    }
    let ratio = accumulated_damage_energy
        .checked_div(failure_energy_tolerance)
        .unwrap_or(Fixed::ONE)
        .clamp(Fixed::ZERO, Fixed::ONE);
    // `ratio` is in [0, 1], so `ONE - ratio` is in [0, 1] and never underflows.
    Fixed::from_bits(Fixed::ONE.to_bits() - ratio.to_bits())
}

/// Fold a batch of per-event insult energies into a total, order- and partition-independent (the
/// `saturating_sum` guarantee), so a centuries-long accumulation cannot depend on how the events were
/// grouped. The value the later slice will store on the part's condition.
pub fn accumulate_damage(increments: impl IntoIterator<Item = Fixed>) -> Fixed {
    Fixed::saturating_sum(increments)
}

// --- Repair and the maintenance economy (slice 2, still inert) ---
//
// The restoring side of the same accumulator: a tissue renews itself at its own turnover rate, funded
// by a maintenance-energy draw on the being's budget. These are pure functions taking reserved
// parameters. Whether the emergent size-longevity relation is clean is NOT a property this slice can
// exhibit or test: none of these functions takes a mass or size input, so the reframed (c) discipline
// (no engineered mass-neutrality; the demand extensive; the slope an emergent output) is a constraint
// on the RE-PIN SLICE's wiring of `energy_available` and `maintenance_demand` from the physiology, not
// something verified here. Slice 2 only supplies the inert math.

/// The energy a part's own tissue restores this tick, in the common kilojoule currency: its turnover
/// rate times its failure-energy tolerance, scaled by how much of the maintenance cost the being can
/// fund. The conversion from a turnover RATE to a repaired ENERGY is through the part's own
/// failure-energy scale so no numeric constant is injected: multiplying `turnover_rate` by
/// `failure_energy_tolerance` (the same kilojoule tolerance [`derive_integrity`] measures damage
/// against) gives the restored energy. Capped at the full tolerance (a tick cannot restore more than
/// the whole failure reserve), and a zero-or-negative tolerance is a part with no failure reserve, so
/// nothing to restore (returns zero), matching [`derive_integrity`]'s guard on the shared tolerance.
///
/// RESERVED: `turnover_rate` is a per-tissue-material value, basis the real measured cell/tissue-renewal
/// rates per tissue (the gating dependency, grounded in tissue-turnover data, never the lifespan it
/// yields). MODELING ASSUMPTION, flagged for the owner review when the reserved value is set: real
/// tissue-turnover data measures a fraction of MASS or CELL COUNT renewed per tick, and this treats that
/// same fraction as the fraction of the failure-ENERGY reserve renewed. Equating a mass/cell-turnover
/// fraction with an energy-reserve-renewal fraction is a modeling choice (an implicit unit-one
/// identification), not a floor law; it avoids inventing a free coefficient (the least-authoring choice)
/// but is not independently grounded, so it is surfaced as its own reserved-with-basis question.
pub fn repair_energy(
    turnover_rate: Fixed,
    failure_energy_tolerance: Fixed,
    funded_fraction: Fixed,
) -> Fixed {
    if failure_energy_tolerance.to_bits() <= 0 {
        return Fixed::ZERO;
    }
    turnover_rate
        .checked_mul(failure_energy_tolerance)
        .and_then(|x| x.checked_mul(funded_fraction))
        .unwrap_or(failure_energy_tolerance)
        .clamp(Fixed::ZERO, failure_energy_tolerance)
}

/// The fraction of full repair the being can fund this tick: `clamp(energy_available / demand, 0, 1)`.
/// The maintenance GATE: when the being cannot pay the full maintenance demand (starving, or a large
/// extensive demand against a smaller budget) repair is throttled proportionally; when it can, repair
/// runs at the full turnover rate. Nothing is engineered mass-neutral: `energy_available` is a fraction
/// of the being's own energy budget and `maintenance_demand` is the extensive cost of the repair, both
/// reserved parameters the re-pin slice derives from the real physiology, so the mass dependence of the
/// duty cycle is whatever those produce. A zero demand is unthrottled (nothing to fund); a zero budget
/// funds nothing. An unrepresentably large ratio saturates in the DIRECTION its sign implies: a large
/// positive surplus is fully funded, a deep negative deficit funds nothing, rather than wrapping or
/// blindly reading a deficit as fully funded.
pub fn maintenance_funded_fraction(energy_available: Fixed, maintenance_demand: Fixed) -> Fixed {
    if maintenance_demand.to_bits() <= 0 {
        return Fixed::ONE;
    }
    // `maintenance_demand` is positive here, so the quotient's sign is `energy_available`'s sign; an
    // overflow (unrepresentable ratio) saturates toward the answer that sign implies.
    let saturated = if energy_available.to_bits() >= 0 {
        Fixed::ONE
    } else {
        Fixed::ZERO
    };
    energy_available
        .checked_div(maintenance_demand)
        .unwrap_or(saturated)
        .clamp(Fixed::ZERO, Fixed::ONE)
}

/// The signed change in a part's accumulated damage energy this tick: the insult energy accrued minus
/// the funded repair energy restored. Positive is net damage, negative is net healing. The later slice
/// applies this to the stored accumulator with a floor at zero (a part cannot heal below intact), so a
/// being whose funded repair exceeds its insults holds its integrity, and one whose insults outrun its
/// repair accumulates toward the first-passage failure. Saturating in raw bits, so it never wraps.
pub fn net_damage_delta(insult_energy: Fixed, repair: Fixed) -> Fixed {
    Fixed::from_bits(insult_energy.to_bits().saturating_sub(repair.to_bits()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::TissueMaterialId;

    fn tissue(hardness: Fixed, sce: Fixed, fe: Fixed) -> TissueMaterial {
        TissueMaterial {
            id: TissueMaterialId(0),
            name: "test".to_string(),
            hardness,
            fracture_strength: Fixed::from_int(100),
            fracture_energy: fe,
            specific_cut_energy: sce,
            elastic_modulus: Fixed::from_int(1000),
            expansion: Fixed::ZERO,
        }
    }

    fn caps() -> InsultCaps {
        InsultCaps {
            wear_coefficient_scaled: Fixed::from_int(1),
            coefficient_scale: Fixed::from_int(1),
            wear_max: Fixed::from_int(1_000_000),
        }
    }

    // --- registry and kernel ---

    #[test]
    fn every_kernel_name_round_trips_and_is_distinct() {
        for &k in InsultKind::ALL.iter() {
            assert_eq!(InsultKind::from_name(k.name()), Some(k));
        }
        let names: Vec<&str> = InsultKind::ALL.iter().map(|k| k.name()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "kernel names must be distinct");
    }

    #[test]
    fn from_kinds_and_from_names_agree_byte_for_byte() {
        let kinds = InsultKind::ALL.to_vec();
        let names: Vec<&str> = kinds.iter().map(|k| k.name()).collect();
        assert_eq!(
            InsultRegistry::from_kinds(&kinds),
            InsultRegistry::from_names(&names).unwrap()
        );
    }

    #[test]
    fn from_names_fails_loud_on_an_unknown_name_and_leaks_no_partial() {
        let err = InsultRegistry::from_names(&["wear", "bogus"]).unwrap_err();
        assert!(err.contains("bogus"), "the error names the offender: {err}");
    }

    #[test]
    fn ids_are_positional_and_reorder_with_the_input() {
        let r = InsultRegistry::from_names(&["wear"]).unwrap();
        assert_eq!(r.defs()[0].id, InsultId(0));
        assert_eq!(r.defs()[0].kind, InsultKind::Wear);
    }

    #[test]
    fn empty_registry_is_empty() {
        let r = InsultRegistry::empty();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    // --- wear energy ---

    #[test]
    fn wear_energy_is_monotonic_in_the_drive_and_zero_at_rest() {
        let mat = tissue(Fixed::from_int(10), Fixed::from_int(2), Fixed::from_int(50));
        let c = caps();
        let at_rest = InsultKind::Wear.energy(
            &mat,
            &InsultDrive::default(),
            &c,
            Fixed::from_int(1_000_000),
        );
        assert_eq!(at_rest, Fixed::ZERO, "no load, no distance, no wear energy");
        let light = InsultKind::Wear.energy(
            &mat,
            &InsultDrive {
                force: Fixed::from_int(100),
                distance: Fixed::from_int(1),
            },
            &c,
            Fixed::from_int(1_000_000),
        );
        let heavy = InsultKind::Wear.energy(
            &mat,
            &InsultDrive {
                force: Fixed::from_int(1000),
                distance: Fixed::from_int(1),
            },
            &c,
            Fixed::from_int(1_000_000),
        );
        assert!(
            heavy > light,
            "more load, strictly more wear energy (inputs far from saturation)"
        );
        assert!(light > Fixed::ZERO, "a real load does real wear");
    }

    #[test]
    fn wear_energy_is_zero_at_rest_even_for_a_zero_hardness_tissue() {
        // A body at rest wears nothing regardless of hardness: the no-drive guard means a zero-hardness
        // (unset/soft) tissue does not inherit wear's "abrades without bound" volume at rest.
        let soft = tissue(Fixed::ZERO, Fixed::from_int(2), Fixed::from_int(50));
        let c = caps();
        let em = Fixed::from_int(1_000_000);
        assert_eq!(
            InsultKind::Wear.energy(&soft, &InsultDrive::default(), &c, em),
            Fixed::ZERO,
            "no load, no slide: no wear energy, even for zero hardness"
        );
    }

    #[test]
    fn wear_energy_saturates_at_the_cap_without_panic() {
        // A soft tissue (low hardness) under an extreme drive drives the worn volume to wear_max and the
        // energy to the energy_max cap; assert saturation actually occurred, not merely that the cap holds.
        let mat = tissue(
            Fixed::from_ratio(1, 1000),
            Fixed::from_int(1_000_000),
            Fixed::from_int(50),
        );
        let c = caps();
        let energy_max = Fixed::from_int(100);
        let drive = InsultDrive {
            force: Fixed::from_int(1_000_000),
            distance: Fixed::from_int(1_000_000),
        };
        let v = laws::wear(
            c.wear_coefficient_scaled,
            c.coefficient_scale,
            drive.force,
            drive.distance,
            mat.hardness,
            c.wear_max,
        );
        assert_eq!(v, c.wear_max, "the extreme drive saturates the worn volume");
        let e = InsultKind::Wear.energy(&mat, &drive, &c, energy_max);
        assert_eq!(e, energy_max, "and the energy saturates at the cap");
    }

    #[test]
    fn wear_energy_inverts_the_cut_model_identity() {
        // The bridge E = V * specific_cut_energy * C_VOL is the inverse of cut_penetrate's
        // depth = E / (specific_cut_energy * contact_area) / C_VOL, so feeding the wear energy back
        // into cut_penetrate recovers depth * contact_area = V. Proven with only public floor laws.
        let sce = Fixed::from_int(3);
        let mat = tissue(Fixed::from_int(10), sce, Fixed::from_int(50));
        let c = caps();
        let drive = InsultDrive {
            force: Fixed::from_int(500),
            distance: Fixed::from_int(2),
        };
        let energy_max = Fixed::from_int(1_000_000);
        let v = laws::wear(
            c.wear_coefficient_scaled,
            c.coefficient_scale,
            drive.force,
            drive.distance,
            mat.hardness,
            c.wear_max,
        );
        let e = InsultKind::Wear.energy(&mat, &drive, &c, energy_max);
        assert!(
            v > Fixed::ZERO && e > Fixed::ZERO,
            "a real cut with real energy"
        );
        let area = Fixed::from_int(4);
        let depth = laws::cut_penetrate(
            Fixed::from_int(1_000_000), // pressure well above hardness so the cut bites
            mat.hardness,
            e,
            sce,
            area,
            Fixed::from_int(1_000_000), // d_max, non-binding
        );
        let recovered_volume = depth.checked_mul(area).unwrap();
        // depth * area recovers V up to the floor's fixed-point rounding; assert a tight bound.
        let diff = Fixed::from_bits((recovered_volume.to_bits() - v.to_bits()).abs());
        assert!(
            diff <= Fixed::from_ratio(1, 1000),
            "the wear energy inverts the cut model: recovered {recovered_volume:?} vs V {v:?}"
        );
    }

    // --- derive_integrity ---

    #[test]
    fn integrity_is_one_at_no_damage_and_zero_at_full_tolerance() {
        let tol = Fixed::from_int(50);
        assert_eq!(derive_integrity(Fixed::ZERO, tol), Fixed::ONE);
        assert_eq!(derive_integrity(tol, tol), Fixed::ZERO);
    }

    #[test]
    fn integrity_is_a_half_at_half_tolerance() {
        let tol = Fixed::from_int(50);
        let half = Fixed::from_ratio(1, 2);
        assert_eq!(derive_integrity(Fixed::from_int(25), tol), half);
    }

    #[test]
    fn integrity_clamps_to_zero_past_full_tolerance_never_negative() {
        let tol = Fixed::from_int(50);
        assert_eq!(derive_integrity(Fixed::from_int(500), tol), Fixed::ZERO);
    }

    #[test]
    fn integrity_of_a_zero_or_negative_tolerance_is_zero_without_panic() {
        assert_eq!(
            derive_integrity(Fixed::from_int(10), Fixed::ZERO),
            Fixed::ZERO
        );
        assert_eq!(
            derive_integrity(Fixed::from_int(10), Fixed::from_int(-5)),
            Fixed::ZERO
        );
    }

    #[test]
    fn integrity_does_not_wrap_on_a_huge_ratio() {
        let tol = Fixed::from_ratio(1, 1000);
        let big = Fixed::from_int(1_000_000);
        assert_eq!(derive_integrity(big, tol), Fixed::ZERO);
    }

    #[test]
    fn integrity_saturates_when_the_ratio_overflows_checked_div() {
        // A ratio beyond Fixed's range makes checked_div return None; the unwrap_or(ONE) fallback must
        // read that as FULL damage (integrity 0), never full integrity. Fixed::MAX over a one-bit
        // tolerance forces the None path the ordinary huge-ratio test does not reach.
        assert_eq!(
            derive_integrity(Fixed::MAX, Fixed::from_bits(1)),
            Fixed::ZERO,
            "an overflowing ratio is a fully-failed part, not a fully-intact one"
        );
    }

    // --- accumulation and determinism ---

    #[test]
    fn accumulate_is_order_and_partition_independent() {
        let a = [
            Fixed::from_int(1),
            Fixed::from_int(2),
            Fixed::from_int(3),
            Fixed::from_int(4),
        ];
        let mut b = a;
        b.reverse();
        assert_eq!(accumulate_damage(a), accumulate_damage(b));
    }

    #[test]
    fn every_public_fn_is_deterministic() {
        let mat = tissue(Fixed::from_int(10), Fixed::from_int(2), Fixed::from_int(50));
        let c = caps();
        let drive = InsultDrive {
            force: Fixed::from_int(300),
            distance: Fixed::from_int(1),
        };
        let em = Fixed::from_int(1_000_000);
        let e1 = InsultKind::Wear.energy(&mat, &drive, &c, em);
        let e2 = InsultKind::Wear.energy(&mat, &drive, &c, em);
        assert_eq!(e1, e2);
        assert_eq!(
            derive_integrity(Fixed::from_int(10), Fixed::from_int(50)),
            derive_integrity(Fixed::from_int(10), Fixed::from_int(50))
        );
    }

    // --- repair and the maintenance economy (slice 2) ---

    #[test]
    fn repair_energy_is_the_turnover_fraction_of_the_tolerance_when_fully_funded() {
        // r = 0.5 per tick (exact in Q32.32), tolerance 50 kJ, fully funded -> 25 kJ restored (through
        // the fracture scale). An exactly-representable rate so the assertion is on the math, not on
        // fixed-point rounding of an inexact fraction like 0.1.
        let tol = Fixed::from_int(50);
        let r = Fixed::from_ratio(1, 2);
        assert_eq!(repair_energy(r, tol, Fixed::ONE), Fixed::from_int(25));
    }

    #[test]
    fn repair_energy_scales_with_the_funded_fraction() {
        let tol = Fixed::from_int(50);
        let r = Fixed::from_ratio(1, 2);
        let half = Fixed::from_ratio(1, 2);
        // 0.5 * 50 * 0.5 = 12.5
        assert_eq!(repair_energy(r, tol, half), Fixed::from_ratio(25, 2));
        assert_eq!(repair_energy(r, tol, Fixed::ZERO), Fixed::ZERO);
    }

    #[test]
    fn repair_energy_caps_at_the_full_tolerance() {
        // A turnover rate above one would restore more than the whole reserve; it caps at the tolerance.
        let tol = Fixed::from_int(50);
        assert_eq!(repair_energy(Fixed::from_int(2), tol, Fixed::ONE), tol);
    }

    #[test]
    fn maintenance_gate_throttles_proportionally_when_the_budget_falls_short() {
        // Half the demand affordable -> half funded; surplus -> fully funded; no demand -> unthrottled;
        // no budget -> nothing funded.
        assert_eq!(
            maintenance_funded_fraction(Fixed::from_int(5), Fixed::from_int(10)),
            Fixed::from_ratio(1, 2)
        );
        assert_eq!(
            maintenance_funded_fraction(Fixed::from_int(20), Fixed::from_int(10)),
            Fixed::ONE
        );
        assert_eq!(
            maintenance_funded_fraction(Fixed::from_int(5), Fixed::ZERO),
            Fixed::ONE
        );
        assert_eq!(
            maintenance_funded_fraction(Fixed::ZERO, Fixed::from_int(10)),
            Fixed::ZERO
        );
    }

    #[test]
    fn net_damage_delta_is_signed_insult_minus_repair() {
        assert_eq!(
            net_damage_delta(Fixed::from_int(10), Fixed::from_int(3)),
            Fixed::from_int(7)
        );
        assert_eq!(
            net_damage_delta(Fixed::from_int(3), Fixed::from_int(10)),
            Fixed::from_int(-7),
            "repair beyond the insult is net healing (the accumulator floors at zero in a later slice)"
        );
        assert_eq!(
            net_damage_delta(Fixed::from_int(5), Fixed::from_int(5)),
            Fixed::ZERO
        );
    }

    #[test]
    fn net_damage_delta_is_zero_when_repair_exactly_matches_insult() {
        // When the funded repair equals the insult, the net delta is zero, so nothing accumulates this
        // tick. This checks the arithmetic identity only; a genuine multi-tick integrity-holding
        // composition is deferred to the slice that wires the accumulator and derive_integrity in sequence.
        let tol = Fixed::from_int(50);
        let insult = Fixed::from_int(25);
        let r = Fixed::from_ratio(1, 2); // 0.5 * 50 = 25 kJ repaired when fully funded (exact)
        let repair = repair_energy(r, tol, Fixed::ONE);
        assert_eq!(repair, insult, "repair matches the insult");
        assert_eq!(
            net_damage_delta(insult, repair),
            Fixed::ZERO,
            "no net delta"
        );
    }

    #[test]
    fn repair_energy_of_a_zero_or_negative_tolerance_is_zero() {
        // A part with no failure reserve has nothing to restore; the guard matches derive_integrity, and
        // it also removes the degenerate clamp(ZERO, negative) case (lo > hi) that would otherwise let a
        // negative "repair" leak into net_damage_delta and INCREASE damage.
        let r = Fixed::from_ratio(1, 2);
        assert_eq!(repair_energy(r, Fixed::ZERO, Fixed::ONE), Fixed::ZERO);
        assert_eq!(
            repair_energy(r, Fixed::from_int(-5), Fixed::ONE),
            Fixed::ZERO
        );
    }

    #[test]
    fn maintenance_gate_funds_nothing_on_a_deep_deficit_that_overflows() {
        // A deep negative energy deficit against a small positive demand overflows checked_div; the
        // sign-aware fallback must fund NOTHING (a starving being does not repair), never blindly read a
        // deficit as fully funded.
        assert_eq!(
            maintenance_funded_fraction(Fixed::MIN, Fixed::from_bits(1)),
            Fixed::ZERO,
            "a deficit that overflows funds nothing, not everything"
        );
        // And a large positive surplus that overflows is fully funded.
        assert_eq!(
            maintenance_funded_fraction(Fixed::MAX, Fixed::from_bits(1)),
            Fixed::ONE
        );
    }
}

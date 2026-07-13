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

//! # civsim-materials: the materials substrate kernel (the Verdict contract)
//!
//! This crate is the spine of the materials substrate buildout (`docs/working/MATERIALS_ORACLE_SPEC.md`,
//! `docs/working/MATERIALS_BUILDOUT_OPENER.md`). It reifies the owner's Verdict kernel contract
//! (`docs/working/VERDICT_KERNEL_CONTRACT.md`): the proposer -> disposer -> freezer selection loop and the
//! Gap/Residual meta-laws, made type-enforced rather than convention-enforced. The KERNEL owns the CONTRACT
//! and the DISCIPLINE; the physics content (the thermochemical and attractor instantiations) lands in plugin
//! modules implementing these traits, so a discipline violation fails to compile rather than relying on review.
//!
//! Slice 1 (this module set) is the contract, physics-free and byte-neutral (nothing consumes it yet):
//!
//! - [`verdict`]: the [`Verdict`] typestate (the Gap Law made unrepresentable-to-violate). A caller reads a
//!   winner only from [`verdict::Decided`] or [`verdict::Trivial`]; when the deciding model cannot separate
//!   the winner from the runner-up within its own resolution (`delta < resolution_s`), the verdict is
//!   [`verdict::Escalate`] (go up the provenance ladder) or, on a collapsed tie the ladder cannot break,
//!   [`verdict::SeededDraw`] (a content-hash-keyed draw into a named contingency slot). Neither carries a
//!   winner, so the resolution-ladder rule is a state that cannot be constructed.
//! - [`contract`]: the [`contract::Proposer`] and [`contract::Disposer`] traits (pure functions of
//!   `(x, E, seed)`), [`contract::preflight`]-shaped [`contract::Preflight`]/[`contract::Validity`]
//!   (representation theorems run before propose), and [`contract::Quench`]/[`contract::RealizedState`] (the
//!   freezer as a fold over the trajectory in the time-marching layer, never inlined into the pure oracle).
//! - [`memo`]: the kernel-provided canonically-iterated [`memo::Memo`] (R-CANON-WALK: key-ordered iteration,
//!   never insertion order), the one place the memoization determinism discipline is enforced.
//! - [`log`]: the coverage-audit [`log::VerdictLog`], so counts-are-queries (the authored-draw count, the
//!   near-degenerate prospecting map, the ceremony-avoidance audit are filters over the log).
//!
//! The two engineering laws the owner's contract names ride on `civsim_core` primitives: candidate
//! canonicalization keys the seeded draw on `content_id` (never enumeration order), and the coverage law is
//! the queryable log. The layering is `core -> physics -> materials -> sim`, acyclic; the `provenance_key`
//! every verdict carries is an opaque `u64` content id that `sim` resolves against the seven-tag joined
//! register, keeping the honesty query where the register lives.

pub mod assemblage;
pub mod contract;
pub mod correlation;
pub mod freezer;
pub mod localized;
pub mod log;
pub mod memo;
pub mod metallic;
pub mod nucleation;
pub mod properties;
pub mod quench;
pub mod thermochemical;
pub mod verdict;

pub use assemblage::{
    realize_assemblage, CoolingPath, DrawContext, EquilibriumExchange, ExchangeKinetics,
    RealizedAssemblage, RealizedExchange,
};
pub use contract::{Disposer, Preflight, Proposer, Quench, RealizedState, Validity};
pub use correlation::{
    route_of_class, CalibrationError, CorrelationClass, CorrelationClassifier, EnergyRoute,
};
pub use localized::LocalizedRoute;
pub use log::{VerdictKind, VerdictLog, VerdictRecord};
pub use memo::Memo;
pub use metallic::MetallicRoute;
pub use nucleation::{
    avrami_grain_size, critical_atom_count, critical_radius_over_spacing, interfacial_energy,
    nucleation_prefactor, nucleation_rate, reduced_driving_force, reduced_interfacial_energy,
    reduced_nucleation_barrier, richards_ratio, zeldovich_factor, NucleationRoute,
};
pub use properties::{
    chen_tse_hardness_gpa, debye_temperature, density_g_per_cm3, poisson_ratio, shear_modulus_gpa,
    youngs_modulus_gpa, PropertyRoute,
};
pub use quench::{
    dodson_closure_temperature, polymorphs_are_thermally_unresolvable, quench_exchange,
    QuenchOutcome,
};
pub use thermochemical::{
    charge_neutral_primitives, mo_viable_diatomics, propose_candidates, BondingHints, Composition,
    Compound, Environment, ThermochemicalDisposer, ThermochemicalProposer,
};
pub use verdict::{
    content_key, dispose, seeded_draw, trivial, Band, Candidate, Decided, Escalate, ProvenanceKey,
    SeededDraw, TieSlot, Trivial, Verdict,
};

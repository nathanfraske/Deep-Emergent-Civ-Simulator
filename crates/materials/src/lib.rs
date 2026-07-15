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
pub mod band_gap;
pub mod contract;
pub mod correlation;
pub mod creep;
pub mod definition;
pub mod electronic;
pub mod freezer;
pub mod harrison;
pub mod localized;
pub mod log;
pub mod magnetism;
pub mod memo;
pub mod metallic;
pub mod nucleation;
pub mod opacity_registry;
pub mod optics;
pub mod properties;
pub mod quench;
pub mod stoner;
pub mod thermochemical;
pub mod verdict;

pub use assemblage::{
    realize_assemblage, CoolingPath, DrawContext, EquilibriumExchange, ExchangeKinetics,
    RealizedAssemblage, RealizedExchange,
};
pub use band_gap::{
    conduction_class_estimated, conduction_class_from_column, conduction_class_measured,
    gap_eigenvalue_provenance, ln_thermal_carrier_activation, ConductionClass, GapGrade,
};
pub use contract::{Disposer, Preflight, Proposer, Quench, RealizedState, Validity};
pub use correlation::{
    route_of_class, CalibrationError, CorrelationClass, CorrelationClassifier, EnergyRoute,
};
pub use creep::{
    creep_dominant_regime, creep_ln_reference_rate, creep_regime_log_rate, creep_selection_gap,
    creep_total_log_rate, CreepComposition, CreepRegime,
};
pub use definition::{
    compound_generation_consistent, require_koopmans_gated, DefinitionMismatch,
    EigenvalueProvenance, Generation,
};
pub use electronic::{
    carrier_density_per_nm3, drude_conductivity_from_tau, drude_conductivity_s_per_m,
    drude_scattering_time_fs, plasma_energy_ev, ElectronicRoute,
};
pub use harrison::{
    bond_covalency, bond_orbital_average_gap_ev, bond_polarity, covalent_energy_v2, eta,
    polar_energy_v3, two_center_matrix_element, TwoCenterBond,
};
pub use localized::LocalizedRoute;
pub use log::{VerdictKind, VerdictLog, VerdictRecord};
pub use magnetism::{
    d_electron_count_3d, hund_local_moment, hund_unpaired_count, low_spin_unpaired_count,
    octahedral_spin_decision, octahedral_spin_moment, spin_only_moment_bohr, OctahedralSpin,
};
pub use memo::Memo;
pub use metallic::MetallicRoute;
pub use nucleation::{
    avrami_grain_size, critical_atom_count, critical_radius_over_spacing, interfacial_energy,
    nucleation_prefactor, nucleation_rate, reduced_driving_force, reduced_interfacial_energy,
    reduced_nucleation_barrier, richards_ratio, zeldovich_factor, NucleationRoute,
};
pub use optics::{
    broadened_step_response, falls_in_observer_window, feature_response_at,
    lifetime_broadening_width_ev, lorentzian_response, optical_energies,
    thermal_broadening_width_ev, OpticalEnergy, OpticalFeature,
};
pub use properties::{
    chen_tse_hardness_gpa, debye_function, debye_heat_capacity_j_per_mol_k, debye_temperature,
    debye_velocity_km_per_s, density_g_per_cm3, grain_boundary_energy_j_per_m2,
    lattice_thermal_conductivity_w_per_m_k, linear_thermal_expansion_per_k,
    operative_shear_strength_gpa, poisson_ratio, shear_modulus_gpa, surface_energy_j_per_m2,
    theoretical_shear_strength_gpa, thermal_diffusivity_m2_per_s,
    volumetric_thermal_expansion_per_k, youngs_modulus_gpa, PropertyRoute,
};
pub use quench::{
    dodson_closure_temperature, polymorphs_are_thermally_unresolvable, quench_exchange,
    QuenchOutcome,
};
pub use stoner::{
    negative_control_gate, stoner_class_from_column, stoner_classify, stoner_product,
    NonmagneticDos, StonerClass, StonerControl,
};
pub use thermochemical::{
    charge_neutral_primitives, mo_viable_diatomics, propose_candidates, BondingHints, Composition,
    Compound, Environment, ThermochemicalDisposer, ThermochemicalProposer,
};
pub use verdict::{
    content_key, dispose, seeded_draw, trivial, Band, Candidate, Decided, Escalate, ProvenanceKey,
    SeededDraw, TieSlot, Trivial, Verdict,
};

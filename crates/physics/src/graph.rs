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

//! The fixed-Rust dataflow contract for the law kernels: the composition-layer hardening
//! (design R-DEEPTECH-PHYSICS, the tier-derivation stage). A law in the registry declares a
//! kernel binding, a set of role-tagged ports, and the axes it produces; the fixed contract
//! here declares what each kernel actually takes and how its output dimension is formed. The
//! loader checks a migrated law against its contract (the binding the old naming convention
//! could not verify, the two-participants-of-one-axis case the flat input list could not
//! express, and the output dimension nothing verified), and derives a law's tier as its depth
//! in the law-output graph rather than an authored stamp.
//!
//! The split is Principle 11: the dimensional exponents, the output-check discriminant, and
//! the port roles are the kernel's own mathematics and are fixed Rust here; a law's wiring
//! (which axis fills which role, which axis it writes) is data that grows with the world. A
//! kernel with no contract is legacy and its laws are not yet checked; a migrated law that
//! binds an unknown kernel fails to load.

use crate::{Dimension, InteractionLaw, PhysicsError, QuantityAxis, Temporal};
use std::collections::BTreeMap;

/// One port of a kernel's contract: the role a law must bind, when the value is read, and the
/// port's signed exponent in the output monomial (0 for a gate or threshold that carries no
/// dimension into the output).
pub struct PortContract {
    /// The role name the law's port must match.
    pub role: &'static str,
    /// Whether the kernel reads this port's value this tick, last tick, or as the tick duration.
    pub temporal: Temporal,
    /// The port's contribution to a `Monomial` output.
    pub exponent: i8,
    /// Whether the kernel folds this role over an open-arity class set (the law's port must then
    /// declare `members` and a fold); a single-value role must be bound by a single port.
    pub variadic: bool,
}

/// How a kernel's primary output dimension is verified against the axis it writes.
pub enum OutputCheck {
    /// The product of each port's axis dimension raised to its exponent equals the output.
    Monomial,
    /// The output shares the dimension of the named role's axis (a same-dimension difference,
    /// fold, or clamp such as a galvanic EMF or the Liebig minimum).
    SameAs(&'static str),
    /// The output is dimensionless (a ratio).
    Dimensionless,
    /// The monomial algebra cannot model this kernel (a dimensional constant carries residual
    /// units, or the kernel forms a rate from a finite difference); the basis is the waiver.
    Asserted(&'static str),
}

/// A kernel's fixed dataflow contract.
pub struct KernelContract {
    /// The value/temporal ports the kernel reads (its reserved caps and constants are not here).
    pub ports: &'static [PortContract],
    /// How the primary output dimension is verified.
    pub output: OutputCheck,
}

const fn cur(role: &'static str, exponent: i8) -> PortContract {
    PortContract {
        role,
        temporal: Temporal::Current,
        exponent,
        variadic: false,
    }
}
const fn prior(role: &'static str, exponent: i8) -> PortContract {
    PortContract {
        role,
        temporal: Temporal::Prior,
        exponent,
        variadic: false,
    }
}
const fn dt(role: &'static str) -> PortContract {
    PortContract {
        role,
        temporal: Temporal::Dt,
        exponent: -1,
        variadic: false,
    }
}
/// A class-set (variadic) port role: the kernel folds an open-arity set of same-dimension axes.
const fn classset(role: &'static str, exponent: i8) -> PortContract {
    PortContract {
        role,
        temporal: Temporal::Current,
        exponent,
        variadic: true,
    }
}

/// The fixed contract for a kernel id, or `None` if the kernel has no contract yet (a legacy
/// kernel whose laws are not checked). All five floors are migrated onto this table (biology,
/// mechanics and materials, fluids, chemistry and optics, electricity and magnetism); the one
/// legacy law is `law.impact`, held back pending its compound kinetic-energy-plus-impulse split.
pub fn kernel_contract(kernel: &str) -> Option<KernelContract> {
    use OutputCheck::*;
    Some(match kernel {
        // === Biology (wave 0) ===
        // The two folds read a class set (the nutrient fractions, the toxin concentrations) as an
        // open-arity variadic port and a handful of single consumer parameters; both report a
        // dimensionless adequacy or harm ratio. edibility composes the two: it reads the produced
        // net-nutrition and net-harm scores, so it derives one tier above them.
        "net_nutrition" => KernelContract {
            ports: const {
                &[
                    classset("supply", 0),
                    cur("requirement", 0),
                    cur("assimilation", 0),
                    cur("fermentation", 0),
                ]
            },
            output: Dimensionless,
        },
        "net_harm" => KernelContract {
            ports: const {
                &[
                    classset("dose", 0),
                    cur("tolerance", 0),
                    cur("hill_exponent", 0),
                ]
            },
            output: Dimensionless,
        },
        "edibility" => KernelContract {
            ports: const {
                &[
                    cur("net_nutrition", 0),
                    cur("net_harm", 0),
                    cur("tolerance_aggregate", 0),
                    cur("dose_aggregate", 0),
                ]
            },
            output: Dimensionless,
        },

        // === Mechanics and materials (wave 1) ===
        // Primary-output contracts: each verifies the law's primary measured consequence. A
        // secondary consequence (a margin, a mechanical advantage, an impulse) and a caller-
        // composed input (a delivered energy, a temperature difference) are a follow-on; where
        // the primary output itself depends on a composed input, the contract is a fail-loud
        // Asserted with that basis, never a lazy skip of a checkable monomial.
        "contact_pressure" => KernelContract {
            ports: const { &[cur("force", 1), cur("contact_area", -1)] },
            output: Monomial,
        },
        "cut_penetrate" => KernelContract {
            ports: const {
                &[
                    cur("hardness", 0),
                    cur("specific_cut_energy", 0),
                    cur("contact_area", 0),
                ]
            },
            output: Asserted("depth = delivered_energy/(specific_cut_energy*area); delivered_energy is the composed output of law.impact, not a registry axis, so the output is not a monomial over the declared ports"),
        },
        "bend_stress" => KernelContract {
            ports: const {
                &[
                    cur("force", 1),
                    cur("span", 1),
                    cur("section_modulus", -1),
                    cur("yield_strength", 0),
                ]
            },
            output: Monomial,
        },
        "axial_stress" => KernelContract {
            ports: const {
                &[
                    cur("force", 1),
                    cur("cross_section", -1),
                    cur("yield_strength", 0),
                ]
            },
            output: Monomial,
        },
        "fracture_onset" => KernelContract {
            ports: const {
                &[
                    cur("fracture_strength", 0),
                    cur("fracture_energy", 0),
                    cur("crack_area", 0),
                ]
            },
            output: Asserted("the stress margin is a same-dimension difference and the energy margin subtracts a composed delivered_energy; not a port monomial"),
        },
        "kinetic_energy" => KernelContract {
            ports: const { &[cur("mass", 1), cur("velocity", 2)] },
            output: Monomial,
        },
        "lever" => KernelContract {
            // The law's primary declared output is the output force F*(effort/load), so load_arm
            // carries exponent -1; the torque and the dimensionless advantage are secondary.
            ports: const {
                &[cur("force", 1), cur("effort_arm", 1), cur("load_arm", -1)]
            },
            output: Monomial,
        },
        "friction" => KernelContract {
            ports: const {
                &[
                    cur("static_coefficient", 0),
                    cur("kinetic_coefficient", 1),
                    cur("normal", 1),
                    cur("tangential", 0),
                    cur("slip_velocity", 0),
                ]
            },
            output: Monomial,
        },
        "reach" => KernelContract {
            ports: const { &[cur("segments", 0)] },
            output: Asserted("an open-arity sum over the segment-length axis; the variadic port kind is a reserved decision, so the fold is not a fixed-arity port monomial"),
        },
        "weight" => KernelContract {
            ports: const { &[cur("mass", 1), cur("gravity", 1)] },
            output: Monomial,
        },
        "power" => KernelContract {
            ports: const { &[cur("force", 1), cur("velocity", 1)] },
            output: Monomial,
        },
        "euler_buckle" => KernelContract {
            ports: const {
                &[
                    cur("modulus", 1),
                    cur("second_moment", 1),
                    cur("effective_length_factor", 0),
                    cur("length", -2),
                ]
            },
            output: Monomial,
        },
        "shear" => KernelContract {
            ports: const {
                &[
                    cur("shear_force", 1),
                    cur("shear_area", -1),
                    cur("independent_shear_strength", 0),
                    cur("yield_strength", 0),
                ]
            },
            output: Monomial,
        },
        "wear" => KernelContract {
            ports: const {
                &[
                    cur("wear_coefficient", 1),
                    cur("force", 1),
                    cur("distance", 1),
                    cur("hardness", -1),
                ]
            },
            output: Monomial,
        },
        "conduction" => KernelContract {
            ports: const {
                &[cur("conductivity", 1), cur("area", 1), cur("path_length", -1)]
            },
            output: Asserted("q = k*(A/L)*dT; the hot and cold temperatures are caller-composed values, not registry axes, so the port product is a thermal conductance, not the flux output"),
        },
        "sensible_energy" => KernelContract {
            ports: const { &[cur("mass", 1), cur("specific_heat", 1)] },
            output: Asserted("Q = m*c*dT; dT is a composed temperature difference supplied by the caller, not a registry axis, so the port product is a heat capacity, not the energy output"),
        },
        "phase_change_energy" => KernelContract {
            ports: const {
                &[
                    cur("mass", 1),
                    cur("specific_heat", 0),
                    cur("transition_temperature", 0),
                    cur("latent_heat", 0),
                ]
            },
            output: Asserted("E = m*c*(T_trans - T_start) + m*L; T_start is composed and the result sums a sensible and a latent term; not a port monomial"),
        },
        "combustion" => KernelContract {
            ports: const {
                &[
                    cur("fuel_value", 1),
                    cur("oxidiser_demand", 0),
                    cur("ignition_temperature", 0),
                    cur("mass", 1),
                ]
            },
            output: Monomial,
        },
        "thermal_stress" => KernelContract {
            ports: const {
                &[
                    cur("modulus", 0),
                    cur("expansion", 0),
                    cur("constraint", 0),
                    cur("fracture_strength", 0),
                ]
            },
            output: Asserted("sigma = E*alpha*dT*constraint; dT is a composed temperature difference, not a registry axis, so the output is not a port monomial"),
        },

        // === Fluids, weather, and acoustics (wave 2) ===
        "hydrostatic_pressure" => KernelContract {
            ports: const { &[cur("density", 1), cur("gravity", 1), cur("height", 1)] },
            output: Monomial,
        },
        "buoyant_force" => KernelContract {
            ports: const { &[cur("density", 1), cur("gravity", 1), cur("volume", 1)] },
            output: Monomial,
        },
        "dynamic_pressure" => KernelContract {
            ports: const { &[cur("density", 1), cur("velocity", 2)] },
            output: Monomial,
        },
        "drag_force" => KernelContract {
            ports: const {
                &[
                    cur("drag_coefficient", 1),
                    cur("density", 1),
                    cur("area", 1),
                    cur("velocity", 2),
                ]
            },
            output: Monomial,
        },
        "aerodynamic_lift" => KernelContract {
            ports: const {
                &[
                    cur("lift_coefficient", 1),
                    cur("density", 1),
                    cur("area", 1),
                    cur("velocity", 2),
                ]
            },
            output: Monomial,
        },
        "reynolds_number" => KernelContract {
            ports: const {
                &[
                    cur("density", 1),
                    cur("velocity", 1),
                    cur("length", 1),
                    cur("viscosity", -1),
                ]
            },
            output: Monomial,
        },
        "laplace_pressure" => KernelContract {
            ports: const { &[cur("surface_tension", 1), cur("radius", -1)] },
            output: Monomial,
        },
        "compressibility" => KernelContract {
            ports: const { &[cur("pressure", 1), cur("bulk_modulus", -1)] },
            output: Monomial,
        },
        "convective_flux" => KernelContract {
            ports: const {
                &[cur("h", 0), cur("area", 0), cur("hot", 0), cur("cold", 0)]
            },
            output: Asserted("q = h*A*|hot-cold|; the two temperatures are a composed difference over therm.temperature, and the flux-versus-per-tick-energy scale is a reserved unit convention"),
        },
        "poiseuille_flow" => KernelContract {
            ports: const {
                &[cur("dp", 0), cur("radius", 0), cur("viscosity", 0), cur("length", 0)]
            },
            output: Asserted("Q = pi*dP*r^4/(8*mu*L); the kernel computes a volumetric flow rate (volume/time) while the law declares volume, the rate-versus-per-tick convention a reserved decision"),
        },
        "speed_of_sound" => KernelContract {
            ports: const { &[cur("bulk_modulus", 0), cur("density", 0)] },
            output: Asserted("c = sqrt(K/rho); the square root halves the exponents, outside the integer monomial algebra"),
        },
        "ideal_gas_density" => KernelContract {
            ports: const { &[cur("pressure", 0), cur("temperature", 0)] },
            output: Asserted("rho = P/(R_s*T); the specific gas constant R_s carries residual dimension, outside the port monomial"),
        },
        "thermal_buoyancy" => KernelContract {
            ports: const {
                &[cur("t_parcel", 0), cur("t_ambient", 0), cur("gravity", 0)]
            },
            output: Asserted("a = g*(T_parcel-T_ambient)/T_ambient; the two temperatures are a composed difference and ratio over therm.temperature, and the declared dimensionless output omits the gravity factor, a reserved convention"),
        },
        "saturation_vapor_pressure" => KernelContract {
            ports: const { &[cur("temperature", 0)] },
            output: Asserted("affine e_s = e_ref + slope*(T-T_ref); an affine form with a dimensional slope and offset, not a monomial"),
        },
        "evaporation_rate" => KernelContract {
            ports: const {
                &[cur("e_ambient", 0), cur("e_saturation", 0), cur("wind", 0)]
            },
            output: Asserted("E = (a + b*|wind|)*(e_s - e_a); a difference of vapor pressures scaled by a wind function with dimensional constants, not a port monomial"),
        },

        // === Chemistry and optics (wave 2) ===
        "reaction" => KernelContract {
            ports: const {
                &[
                    cur("products_sum", 0),
                    cur("reactants_sum", 0),
                    cur("temperature", 0),
                ]
            },
            output: SameAs("products_sum"),
        },
        "corrosion" => KernelContract {
            ports: const {
                &[
                    cur("fluid_potential", 0),
                    cur("material_potential", 0),
                    cur("susceptibility", 0),
                    cur("acidity_factor", 0),
                ]
            },
            output: Asserted("driving = (fluid_potential - material_potential)*susceptibility*acidity; a difference of potentials, and the declared dimensionless output treats the susceptibility as carrying the inverse-voltage scale, a reserved question"),
        },
        "carnot_limit" => KernelContract {
            ports: const { &[cur("hot", 0), cur("cold", 0)] },
            output: Dimensionless,
        },
        "dissolution" => KernelContract {
            ports: const { &[cur("solute_affinity", 0)] },
            output: Dimensionless,
        },
        "radiant_emission" => KernelContract {
            ports: const {
                &[
                    cur("emissivity", 0),
                    cur("area", 0),
                    cur("t_hot", 0),
                    cur("t_cold", 0),
                ]
            },
            output: Asserted("j = emissivity*sigma*(T_hot^4 - T_cold^4); the Stefan-Boltzmann sigma carries residual dimension and the fourth-power difference is not a monomial"),
        },
        "wien_peak" => KernelContract {
            ports: const { &[cur("temperature", 0)] },
            output: Asserted("lambda = wien_b/T; Wien's displacement constant carries residual dimension m*K, outside the port monomial"),
        },
        "inverse_square_falloff" => KernelContract {
            ports: const { &[cur("power", 1), cur("distance", -2)] },
            output: Monomial,
        },
        "interface_split" => KernelContract {
            ports: const { &[cur("reflectance", 0), cur("transmittance", 0)] },
            output: Asserted("the reflected/absorbed/transmitted split is a fraction of the composed incident flux, not a monomial over the dimensionless partition coefficients"),
        },
        "optical_depth" => KernelContract {
            ports: const { &[cur("absorption_coefficient", 1), cur("path", 1)] },
            output: Monomial,
        },
        "refractive_contrast" => KernelContract {
            ports: const { &[cur("n1", 0), cur("n2", 0)] },
            output: Dimensionless,
        },
        "radiative_equilibrium" => KernelContract {
            ports: const { &[cur("emissivity", 0)] },
            output: Asserted("T_eq = (E_abs/(emissivity*sigma))^(1/4); a fourth root with the dimensional Stefan-Boltzmann sigma, outside the integer monomial algebra"),
        },

        // === Electricity and magnetism (wave 3) ===
        "coulomb_force" => KernelContract {
            ports: const { &[cur("q1", 1), cur("q2", 1), cur("r", -2)] },
            output: Asserted("F = k*|q1||q2|/r^2; the Coulomb constant k carries the residual dimension N*m^2/C^2, outside the port monomial"),
        },
        "ohm_voltage" => KernelContract {
            ports: const { &[cur("current", 1), cur("resistance", 1)] },
            output: Monomial,
        },
        "circuit_current" => KernelContract {
            ports: const { &[cur("emf", 1), cur("r_total", -1)] },
            output: Monomial,
        },
        "power_dissipation" => KernelContract {
            ports: const { &[cur("current", 1), cur("voltage", 1)] },
            output: Monomial,
        },
        "capacitor_energy" => KernelContract {
            ports: const { &[cur("capacitance", 1), cur("voltage", 2)] },
            output: Monomial,
        },
        "battery_emf" => KernelContract {
            ports: const { &[cur("cathode", 1), cur("anode", 1)] },
            output: SameAs("cathode"),
        },
        "resistance" => KernelContract {
            ports: const { &[cur("resistivity", 1), cur("length", 1), cur("area", -1)] },
            output: Monomial,
        },
        "solenoid_field" => KernelContract {
            ports: const { &[cur("permeability", 1), cur("current", 1)] },
            output: Asserted("B = mu_0*mu_r*n*I; the vacuum permeability and the turn density carry the residual dimension, outside the port monomial"),
        },
        "flux_linkage" => KernelContract {
            ports: const { &[cur("flux_density", 1), cur("area", 1)] },
            output: Monomial,
        },
        "motor_force" => KernelContract {
            ports: const { &[cur("flux_density", 1), cur("current", 1), cur("length", 1)] },
            output: Monomial,
        },
        "lorentz_force" => KernelContract {
            ports: const { &[cur("charge", 1), cur("velocity", 1), cur("flux_density", 1)] },
            output: Monomial,
        },
        "dipole_torque" => KernelContract {
            ports: const { &[cur("moment", 1), cur("flux_density", 1)] },
            output: Monomial,
        },
        "faraday_emf" => KernelContract {
            ports: const { &[cur("flux_now", 1), prior("flux_prev", 1), dt("dt")] },
            output: Asserted("EMF = -N*dPhi/dt; the finite difference of two same-dimension flux samples over the tick is a rate, not a port monomial"),
        },
        "inductive_emf" => KernelContract {
            ports: const { &[cur("current_now", 1), prior("current_prev", 1), dt("dt")] },
            output: Asserted("EMF = -L*dI/dt; the finite difference of two same-dimension current samples over the tick is a rate, not a port monomial"),
        },
        "inductor_energy" => KernelContract {
            ports: const { &[cur("inductance", 1), cur("current", 2)] },
            output: Monomial,
        },
        _ => return None,
    })
}

/// Raise a dimension to an integer power: each base exponent is scaled, so `dim^e` is a pure
/// integer operation with no rounding.
fn dim_pow(d: Dimension, e: i8) -> Dimension {
    Dimension {
        length: d.length * e,
        mass: d.mass * e,
        time: d.time * e,
        temperature: d.temperature * e,
        current: d.current * e,
    }
}

/// The tick-duration dimension: time to the first power.
const TICK_DIMENSION: Dimension = Dimension::TIME;

/// Check a migrated law against its kernel contract: the binding (every contract role is bound
/// exactly once, no role is unbound, no extra role is declared), the temporal agreement, and
/// the dimensional reachability of the produced axis (or the declared output dimension when the
/// law produces no axis). A legacy law (empty kernel) is not checked here.
pub fn check_law(
    law: &InteractionLaw,
    axes: &BTreeMap<String, QuantityAxis>,
) -> Result<(), PhysicsError> {
    if law.kernel.is_empty() {
        return Ok(());
    }
    let contract = kernel_contract(&law.kernel).ok_or_else(|| PhysicsError::UnknownKernel {
        law: law.id.clone(),
        kernel: law.kernel.clone(),
    })?;

    // Binding: the law's port roles must be exactly the contract's roles, each once, agreeing on
    // when the value is read and on whether the role is a single value or an open-arity class set.
    let mut law_roles: BTreeMap<&str, &crate::LawPort> = BTreeMap::new();
    for port in &law.ports {
        if law_roles.insert(port.role.as_str(), port).is_some() {
            return Err(PhysicsError::PortContractMismatch {
                law: law.id.clone(),
                detail: format!("role '{}' is declared more than once", port.role),
            });
        }
    }
    for pc in contract.ports {
        match law_roles.remove(pc.role) {
            None => {
                return Err(PhysicsError::PortContractMismatch {
                    law: law.id.clone(),
                    detail: format!(
                        "kernel '{}' needs a port for role '{}'",
                        law.kernel, pc.role
                    ),
                });
            }
            Some(port) => {
                if port.temporal != pc.temporal {
                    return Err(PhysicsError::PortContractMismatch {
                        law: law.id.clone(),
                        detail: format!(
                            "role '{}' reads at {:?}, but kernel '{}' reads it at {:?}",
                            pc.role, port.temporal, law.kernel, pc.temporal
                        ),
                    });
                }
                let is_classset = !port.members.is_empty();
                if is_classset != pc.variadic {
                    return Err(PhysicsError::PortContractMismatch {
                        law: law.id.clone(),
                        detail: if pc.variadic {
                            format!(
                                "role '{}' of kernel '{}' folds a class set, but the port declares a single axis",
                                pc.role, law.kernel
                            )
                        } else {
                            format!(
                                "role '{}' of kernel '{}' reads a single value, but the port declares a class set",
                                pc.role, law.kernel
                            )
                        },
                    });
                }
            }
        }
    }
    if let Some((extra, _)) = law_roles.into_iter().next() {
        return Err(PhysicsError::PortContractMismatch {
            law: law.id.clone(),
            detail: format!("role '{extra}' is not a port of kernel '{}'", law.kernel),
        });
    }

    // Dimensional reachability of the primary output.
    let target = match law.produces.first() {
        Some(axis_id) => match axes.get(axis_id) {
            Some(a) => a.dimension,
            None => {
                return Err(PhysicsError::UnknownAxis {
                    context: law.id.clone(),
                    axis: axis_id.clone(),
                });
            }
        },
        None => law.output_dimension,
    };
    check_output_dimension(law, &contract, axes, target)
}

fn port_axis_dimension(
    law: &InteractionLaw,
    role: &str,
    axes: &BTreeMap<String, QuantityAxis>,
) -> Result<Dimension, PhysicsError> {
    let port = law
        .ports
        .iter()
        .find(|p| p.role == role)
        .expect("binding check ran first, so the role is present");
    if port.temporal == Temporal::Dt {
        return Ok(TICK_DIMENSION);
    }
    // A class-set port's dimension is its members' shared dimension (validate() has already
    // proven the members exist and agree), so a fold contributes that one dimension to the
    // monomial; a single port contributes its axis's dimension.
    let axis_id = if port.members.is_empty() {
        &port.axis
    } else {
        &port.members[0]
    };
    axes.get(axis_id)
        .map(|a| a.dimension)
        .ok_or_else(|| PhysicsError::UnknownAxis {
            context: law.id.clone(),
            axis: axis_id.clone(),
        })
}

fn check_output_dimension(
    law: &InteractionLaw,
    contract: &KernelContract,
    axes: &BTreeMap<String, QuantityAxis>,
    target: Dimension,
) -> Result<(), PhysicsError> {
    match &contract.output {
        OutputCheck::Asserted(_) => Ok(()),
        OutputCheck::Dimensionless => {
            if target.is_dimensionless() {
                Ok(())
            } else {
                Err(PhysicsError::DimensionUnreachable {
                    law: law.id.clone(),
                    detail: format!("declared output {target:?} is not dimensionless"),
                })
            }
        }
        OutputCheck::SameAs(role) => {
            let d = port_axis_dimension(law, role, axes)?;
            if d == target {
                Ok(())
            } else {
                Err(PhysicsError::DimensionUnreachable {
                    law: law.id.clone(),
                    detail: format!(
                        "output {target:?} does not match role '{role}' dimension {d:?}"
                    ),
                })
            }
        }
        OutputCheck::Monomial => {
            let mut acc = Dimension::DIMENSIONLESS;
            for pc in contract.ports {
                let d = port_axis_dimension(law, pc.role, axes)?;
                acc = acc * dim_pow(d, pc.exponent);
            }
            if acc == target {
                Ok(())
            } else {
                Err(PhysicsError::DimensionUnreachable {
                    law: law.id.clone(),
                    detail: format!(
                        "the port monomial reduces to {acc:?}, but the declared output is {target:?}"
                    ),
                })
            }
        }
    }
}

/// Derive each law's tier as its longest-path depth in the law-output graph: a ground axis
/// (produced by no law) is tier 0, a law is one plus the maximum tier over the axes it reads,
/// and a produced axis inherits its producing law's tier. A cycle is a load error. This is the
/// derived layering that fills the empty middle the authored stamps left, computed rather than
/// asserted. Only migrated laws (those that declare `produces`) contribute edges; a legacy law
/// reads ground axes and sits at tier 1.
pub fn derive_tiers(
    laws: &BTreeMap<String, InteractionLaw>,
) -> Result<BTreeMap<String, u32>, PhysicsError> {
    // producer: axis id -> a law id that writes it.
    let mut producer: BTreeMap<&str, &str> = BTreeMap::new();
    for law in laws.values() {
        for axis in &law.produces {
            producer.insert(axis.as_str(), law.id.as_str());
        }
    }

    let mut law_tier: BTreeMap<String, u32> = BTreeMap::new();
    // visiting: on the current DFS stack (cycle detection); done: finalized.
    let mut visiting: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // Iterative-safe recursion via an explicit helper closure is awkward in Rust; use a
    // recursive fn with the maps threaded through.
    fn tier_of(
        law_id: &str,
        laws: &BTreeMap<String, InteractionLaw>,
        producer: &BTreeMap<&str, &str>,
        law_tier: &mut BTreeMap<String, u32>,
        visiting: &mut std::collections::BTreeSet<String>,
    ) -> Result<u32, PhysicsError> {
        if let Some(t) = law_tier.get(law_id) {
            return Ok(*t);
        }
        if !visiting.insert(law_id.to_string()) {
            return Err(PhysicsError::CyclicLawGraph(law_id.to_string()));
        }
        let law = &laws[law_id];
        let mut max_input_tier: u32 = 0;
        for axis in &law.inputs {
            let axis_tier = match producer.get(axis.as_str()) {
                // A produced axis inherits its producing law's tier (unless the law reads its
                // own output, which self-cycles and is caught by the visiting set).
                Some(prod) => tier_of(prod, laws, producer, law_tier, visiting)?,
                // A ground axis, produced by no law.
                None => 0,
            };
            max_input_tier = max_input_tier.max(axis_tier);
        }
        visiting.remove(law_id);
        let t = max_input_tier + 1;
        law_tier.insert(law_id.to_string(), t);
        Ok(t)
    }

    for id in laws.keys() {
        tier_of(id, laws, &producer, &mut law_tier, &mut visiting)?;
    }
    Ok(law_tier)
}

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
    }
}
const fn prior(role: &'static str, exponent: i8) -> PortContract {
    PortContract {
        role,
        temporal: Temporal::Prior,
        exponent,
    }
}
const fn dt(role: &'static str) -> PortContract {
    PortContract {
        role,
        temporal: Temporal::Dt,
        exponent: -1,
    }
}

/// The fixed contract for a kernel id, or `None` if the kernel has no contract yet (a legacy
/// kernel whose laws are not checked). Electricity and magnetism (wave 3) is migrated; the
/// other floors bind their kernels as they are migrated onto this table.
pub fn kernel_contract(kernel: &str) -> Option<KernelContract> {
    use OutputCheck::*;
    Some(match kernel {
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

    // Binding: the law's port roles must be exactly the contract's roles, each once.
    let mut law_roles: BTreeMap<&str, &Temporal> = BTreeMap::new();
    for port in &law.ports {
        if law_roles
            .insert(port.role.as_str(), &port.temporal)
            .is_some()
        {
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
            Some(temporal) => {
                if *temporal != pc.temporal {
                    return Err(PhysicsError::PortContractMismatch {
                        law: law.id.clone(),
                        detail: format!(
                            "role '{}' reads at {:?}, but kernel '{}' reads it at {:?}",
                            pc.role, temporal, law.kernel, pc.temporal
                        ),
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
    axes.get(&port.axis)
        .map(|a| a.dimension)
        .ok_or_else(|| PhysicsError::UnknownAxis {
            context: law.id.clone(),
            axis: port.axis.clone(),
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

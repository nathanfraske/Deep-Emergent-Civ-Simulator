// Copyright 2026 Nathan M. Fraske
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Retired biology graph contracts, composed with the root abiotic contract table.

use crate::{InteractionLaw, PhysicsError, QuantityAxis, Temporal};
use civsim_physics_abiotic::graph::{KernelContract, OutputCheck, PortContract};
use std::collections::BTreeMap;

pub use civsim_physics_abiotic::graph::{check_law_with_contract, derive_tiers};

const fn cur(role: &'static str) -> PortContract {
    PortContract {
        role,
        temporal: Temporal::Current,
        exponent: 0,
        variadic: false,
    }
}

const fn classset(role: &'static str) -> PortContract {
    PortContract {
        role,
        temporal: Temporal::Current,
        exponent: 0,
        variadic: true,
    }
}

/// A retired biology contract, or `None` when the kernel belongs to the root abiotic table.
pub fn legacy_kernel_contract(kernel: &str) -> Option<KernelContract> {
    use OutputCheck::Dimensionless;
    Some(match kernel {
        "net_nutrition" => KernelContract {
            ports: const {
                &[
                    classset("supply"),
                    cur("requirement"),
                    cur("assimilation"),
                    cur("fermentation"),
                ]
            },
            output: Dimensionless,
        },
        "net_harm" => KernelContract {
            ports: const { &[classset("dose"), cur("tolerance"), cur("hill_exponent")] },
            output: Dimensionless,
        },
        "edibility" => KernelContract {
            ports: const {
                &[
                    cur("net_nutrition"),
                    cur("net_harm"),
                    cur("tolerance_aggregate"),
                    cur("dose_aggregate"),
                ]
            },
            output: Dimensionless,
        },
        _ => return None,
    })
}

/// The combined compatibility contract table.
pub fn kernel_contract(kernel: &str) -> Option<KernelContract> {
    civsim_physics_abiotic::graph::kernel_contract(kernel)
        .or_else(|| legacy_kernel_contract(kernel))
}

/// Validate a law against the combined compatibility table.
pub fn check_law(
    law: &InteractionLaw,
    axes: &BTreeMap<String, QuantityAxis>,
) -> Result<(), PhysicsError> {
    civsim_physics_abiotic::graph::check_law_with_contract(law, axes, kernel_contract(&law.kernel))
}

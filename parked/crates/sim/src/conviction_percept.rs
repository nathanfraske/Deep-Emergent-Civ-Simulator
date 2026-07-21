// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.

//! The conviction-percept registry (Prereq B for the learned experience-to-conviction coupling,
//! `docs/working/OWNER_DECISIONS_LOG.md` R2): which of a being's own convictions (axiom-axis stances) are
//! exposed to its behaviour controller as input channels, so the evolved controller CAN learn to weight a
//! conviction in what it does. It is the substrate that lets convictions bias EMBODIED behaviour without an
//! authored "this conviction implies that action" rule: the stance is a raw input the controller reads, and
//! whether and how a conviction sways behaviour EMERGES from selection over the evolved weight (founder-zero,
//! so a conviction moves nothing until selection lifts a weight off zero, the emergent pattern the feature,
//! appetitive, material, and attraction percept blocks already established, Principle 8).
//!
//! Sibling of the [`crate::percept::PerceptRegistry`], [`crate::material_percept::MaterialPerceptRegistry`],
//! and the appetitive block: the mechanism (a percept block the controller reads) is fixed Rust, the
//! membership (which conviction axes a world exposes) is data (Principle 11), and it is EMPTY by default, so a
//! world that exposes no conviction feeds the controller no conviction block and every run hash is unchanged
//! (opt-in, byte-neutral). Keys on [`crate::axiom::AxiomAxisId`], the being's own conviction axis, never a
//! named institution or religion (the Steering Audit bites here): the axis is just an id, and what it means is
//! the world's data.

use crate::axiom::AxiomAxisId;

/// The set of a being's conviction axes (axiom-axis stances) exposed to its behaviour controller as input
/// channels, data-defined and extensible. EMPTY by default, so a world that exposes no conviction leaves the
/// controller layout and every run hash unchanged (the conviction percept is opt-in). The order is the
/// canonical conviction-block order; a world declares the convictions its beings can act on as data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConvictionPerceptRegistry {
    axes: Vec<AxiomAxisId>,
}

impl ConvictionPerceptRegistry {
    /// An empty registry: no conviction is exposed, so the controller grows no conviction block and the run is
    /// bit-identical to a world without the conviction-percept substrate. The default and the opt-out.
    pub fn empty() -> ConvictionPerceptRegistry {
        ConvictionPerceptRegistry { axes: Vec::new() }
    }

    /// A registry over an explicit ordered list of conviction axes. The order is the canonical
    /// conviction-block order; a world declares which of its beings' convictions can bias behaviour as data.
    pub fn from_axes(axes: &[AxiomAxisId]) -> ConvictionPerceptRegistry {
        ConvictionPerceptRegistry {
            axes: axes.to_vec(),
        }
    }

    /// The conviction axes in canonical order (the conviction-block channel order).
    pub fn axes(&self) -> &[AxiomAxisId] {
        &self.axes
    }

    /// The number of conviction channels (the width the controller's conviction input block adds).
    pub fn len(&self) -> usize {
        self.axes.len()
    }

    /// Whether the registry exposes no conviction (the opt-out: the controller grows no conviction block).
    pub fn is_empty(&self) -> bool {
        self.axes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_conviction_registry_is_the_byte_neutral_opt_out() {
        let empty = ConvictionPerceptRegistry::empty();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty, ConvictionPerceptRegistry::default());
    }

    #[test]
    fn a_declared_registry_carries_its_axes_in_order() {
        let reg = ConvictionPerceptRegistry::from_axes(&[AxiomAxisId(2), AxiomAxisId(0)]);
        assert_eq!(reg.len(), 2);
        assert!(!reg.is_empty());
        // Canonical order is the declared order (the world's data), not sorted: the block channel order
        // follows the registry, and a seed or a reader reads the axes accessor rather than assuming a sort.
        assert_eq!(reg.axes(), &[AxiomAxisId(2), AxiomAxisId(0)]);
    }
}

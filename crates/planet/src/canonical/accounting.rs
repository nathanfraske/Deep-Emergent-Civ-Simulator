//! Exact value and conservation record shapes for the canonical transcript.
//!
//! Constructors remain inside the canonical implementation. Public consumers
//! can inspect records, but cannot bind an arbitrary magnitude to an admitted
//! identity.

use super::{floor_magnitudes::AuditedMagnitude, BodyId, EventId, ReservoirId, Stage};
use civsim_ledger::{DerivationExhaustionReceipt, Provenance, Tier};
use civsim_units::constants::ScaledConstant;
use civsim_units::fundamentals::{Composite, Fundamental, SiDimension};

/// Stable rule used to project a source rational into a scaled integer.
pub const FLOOR_PROJECTION_RULE_ID: &str = "exact_rational.nearest_ties_to_even.v1";

/// Stable rule used by current composite floor laws.
pub const SI_EXECUTION_DERIVATION_ID: &str =
    "projected_inputs.exact_rational.nearest_ties_to_even.v1";

/// An exact signed integer mantissa with an explicit binary scale.
///
/// The represented value is `bits * 2^-scale_bits`. This type has no public
/// constructor. The current transcript slice creates it only from sealed,
/// typed floor magnitudes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExactScaledValue {
    bits: i128,
    scale_bits: u32,
}

impl ExactScaledValue {
    pub(super) fn from_audited<Kind: Copy>(value: AuditedMagnitude<Kind>) -> Self {
        Self {
            bits: value.bits(),
            scale_bits: value.scale_bits(),
        }
    }

    pub(super) fn from_repository_constant(value: ScaledConstant) -> Self {
        Self {
            bits: value.bits(),
            scale_bits: value.scale_bits(),
        }
    }

    /// Signed integer mantissa.
    pub const fn bits(self) -> i128 {
        self.bits
    }

    /// Number of binary fractional places in the mantissa.
    pub const fn scale_bits(self) -> u32 {
        self.scale_bits
    }
}

/// Named law and complete direct input ancestry for one computed value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawAncestry {
    law_id: String,
    expression: Option<String>,
    evaluation_id: Option<&'static str>,
    input_ids: Vec<String>,
}

impl LawAncestry {
    pub(super) fn sealed_execution_derivation(
        law_id: String,
        expression: &str,
        input_ids: Vec<String>,
    ) -> Self {
        Self {
            law_id,
            expression: Some(expression.to_owned()),
            evaluation_id: Some(SI_EXECUTION_DERIVATION_ID),
            input_ids,
        }
    }

    /// Stable identity of the law that computed the output.
    pub fn law_id(&self) -> &str {
        &self.law_id
    }

    /// Readable exact relation when the owning law catalog provides one.
    pub fn expression(&self) -> Option<&str> {
        self.expression.as_deref()
    }

    /// Stable arithmetic and terminal-rounding contract for this law.
    pub const fn evaluation_id(&self) -> Option<&'static str> {
        self.evaluation_id
    }

    /// Direct value identities read by the law, in its declared order.
    pub fn input_ids(&self) -> &[String] {
        &self.input_ids
    }
}

/// Source custody and numerical projection receipt for one `[M]` floor leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementEvidence {
    source_id: &'static str,
    source_sha256: &'static str,
    source_anchor: &'static str,
    source_decimal: &'static str,
    uncertainty_kind: &'static str,
    uncertainty_decimal: &'static str,
    projection_rule_id: &'static str,
    projection_max_abs_error: ExactScaledValue,
}

impl MeasurementEvidence {
    fn sealed(fundamental: &Fundamental, projected: ExactScaledValue) -> Self {
        Self {
            source_id: fundamental.source_id,
            source_sha256: fundamental.source_sha256,
            source_anchor: fundamental.source_anchor,
            source_decimal: fundamental.value,
            uncertainty_kind: fundamental.uncertainty.kind_id(),
            uncertainty_decimal: fundamental.uncertainty.decimal(),
            projection_rule_id: FLOOR_PROJECTION_RULE_ID,
            projection_max_abs_error: ExactScaledValue {
                bits: 1,
                scale_bits: projected
                    .scale_bits
                    .checked_add(1)
                    .expect("the sealed floor projection scale leaves room for a half-ULP bound"),
            },
        }
    }

    pub const fn source_id(&self) -> &'static str {
        self.source_id
    }

    pub const fn source_sha256(&self) -> &'static str {
        self.source_sha256
    }

    pub const fn source_anchor(&self) -> &'static str {
        self.source_anchor
    }

    pub const fn source_decimal(&self) -> &'static str {
        self.source_decimal
    }

    pub const fn uncertainty_kind(&self) -> &'static str {
        self.uncertainty_kind
    }

    pub const fn uncertainty_decimal(&self) -> &'static str {
        self.uncertainty_decimal
    }

    pub const fn projection_rule_id(&self) -> &'static str {
        self.projection_rule_id
    }

    pub const fn projection_max_abs_error(&self) -> ExactScaledValue {
        self.projection_max_abs_error
    }
}

/// One exact value with its accounting and derivation ancestry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactValueRecord {
    quantity_id: String,
    unit_id: String,
    dimension: SiDimension,
    value: ExactScaledValue,
    tier: Tier,
    provenance: Provenance,
    measurement: Option<MeasurementEvidence>,
    exhaustion: Option<DerivationExhaustionReceipt>,
    ancestry: Option<LawAncestry>,
}

impl ExactValueRecord {
    pub(super) fn sealed_measured_floor(
        quantity_id: String,
        fundamental: &Fundamental,
        value: ExactScaledValue,
        tier: Tier,
        provenance: Provenance,
        exhaustion: &DerivationExhaustionReceipt,
    ) -> Self {
        debug_assert_eq!(provenance, Provenance::Measured);
        Self {
            quantity_id,
            unit_id: fundamental.unit.to_owned(),
            dimension: fundamental.dimension,
            value,
            tier,
            provenance,
            measurement: Some(MeasurementEvidence::sealed(fundamental, value)),
            exhaustion: Some(exhaustion.clone()),
            ancestry: None,
        }
    }

    pub(super) fn sealed_derived_value(
        quantity_id: String,
        composite: &Composite,
        value: ExactScaledValue,
        tier: Tier,
        provenance: Provenance,
        ancestry: LawAncestry,
    ) -> Self {
        debug_assert_eq!(provenance, Provenance::Derived);
        debug_assert!(!ancestry.input_ids.is_empty());
        Self {
            quantity_id,
            unit_id: composite.unit.to_owned(),
            dimension: composite.dimension,
            value,
            tier,
            provenance,
            measurement: None,
            exhaustion: None,
            ancestry: Some(ancestry),
        }
    }

    /// Stable quantity identity.
    pub fn quantity_id(&self) -> &str {
        &self.quantity_id
    }

    /// Unit or dimension identity in which the exact bits are expressed.
    pub fn unit_id(&self) -> &str {
        &self.unit_id
    }

    /// Typed SI base-dimension exponents in the canonical base order.
    pub const fn dimension(&self) -> SiDimension {
        self.dimension
    }

    /// Exact scaled integer representation.
    pub const fn value(&self) -> ExactScaledValue {
        self.value
    }

    /// Ledger tier accounting for this value.
    pub const fn tier(&self) -> Tier {
        self.tier
    }

    /// One of the seven canonical provenance types.
    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }

    /// Source and projection evidence for a measured floor leaf.
    pub const fn measurement(&self) -> Option<&MeasurementEvidence> {
        self.measurement.as_ref()
    }

    /// Derive-first, Buckingham-Pi, Gap-Law, and Residual-Law evidence for an
    /// irreducible initial-floor leaf.
    pub const fn exhaustion(&self) -> Option<&DerivationExhaustionReceipt> {
        self.exhaustion.as_ref()
    }

    /// Named law and direct inputs for a derived value.
    pub fn ancestry(&self) -> Option<&LawAncestry> {
        self.ancestry.as_ref()
    }
}

/// A holder participating in a conservation transfer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConservationHolderId {
    Body(BodyId),
    Reservoir(ReservoirId),
}

impl ConservationHolderId {
    /// Stable transcript variant name, independent of Rust debug formatting.
    pub const fn kind_id(&self) -> &'static str {
        match self {
            Self::Body(_) => "body",
            Self::Reservoir(_) => "reservoir",
        }
    }

    /// Opaque identity assigned by the canonical transcript.
    pub fn identity(&self) -> &str {
        match self {
            Self::Body(body) => body.as_str(),
            Self::Reservoir(reservoir) => reservoir.as_str(),
        }
    }
}

/// One exact debit or credit in a transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferLeg {
    holder: ConservationHolderId,
    amount: ExactScaledValue,
}

impl TransferLeg {
    /// Body or reservoir receiving the debit or credit.
    pub fn holder(&self) -> &ConservationHolderId {
        &self.holder
    }

    /// Exact nonnegative amount assigned to this leg.
    pub const fn amount(&self) -> ExactScaledValue {
        self.amount
    }
}

/// Physical topology of one conservation event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransferOperation {
    Move,
    Split,
    Merge,
    BoundaryIn,
    BoundaryOut,
}

impl TransferOperation {
    /// Stable transcript spelling.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Split => "split",
            Self::Merge => "merge",
            Self::BoundaryIn => "boundary_in",
            Self::BoundaryOut => "boundary_out",
        }
    }
}

/// Exact balance around one transfer event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConservationBalance {
    before: ExactScaledValue,
    debited: ExactScaledValue,
    credited: ExactScaledValue,
    boundary_net: ExactScaledValue,
    after: ExactScaledValue,
    residual: ExactScaledValue,
}

impl ConservationBalance {
    pub const fn before(&self) -> ExactScaledValue {
        self.before
    }

    pub const fn debited(&self) -> ExactScaledValue {
        self.debited
    }

    pub const fn credited(&self) -> ExactScaledValue {
        self.credited
    }

    pub const fn boundary_net(&self) -> ExactScaledValue {
        self.boundary_net
    }

    pub const fn after(&self) -> ExactScaledValue {
        self.after
    }

    /// Must be exact zero for a closed transfer.
    pub const fn residual(&self) -> ExactScaledValue {
        self.residual
    }
}

/// Append-only evidence for one exact conserved transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferReceipt {
    stage: Stage,
    quantity_id: String,
    unit_id: String,
    operation: TransferOperation,
    sources: Vec<TransferLeg>,
    destinations: Vec<TransferLeg>,
    balance: ConservationBalance,
    law: LawAncestry,
    input_events: Vec<EventId>,
    tier: Tier,
    provenance: Provenance,
}

impl TransferReceipt {
    pub const fn stage(&self) -> Stage {
        self.stage
    }

    pub fn quantity_id(&self) -> &str {
        &self.quantity_id
    }

    pub fn unit_id(&self) -> &str {
        &self.unit_id
    }

    pub const fn operation(&self) -> TransferOperation {
        self.operation
    }

    pub fn sources(&self) -> &[TransferLeg] {
        &self.sources
    }

    pub fn destinations(&self) -> &[TransferLeg] {
        &self.destinations
    }

    pub const fn balance(&self) -> &ConservationBalance {
        &self.balance
    }

    pub const fn law(&self) -> &LawAncestry {
        &self.law
    }

    pub fn input_events(&self) -> &[EventId] {
        &self.input_events
    }

    pub const fn tier(&self) -> Tier {
        self.tier
    }

    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservation_holder_wire_names_do_not_depend_on_debug_output() {
        let event = EventId::generated(4);
        let body = ConservationHolderId::Body(BodyId::generated(event, 2));
        let reservoir = ConservationHolderId::Reservoir(ReservoirId::generated(event, 3));

        assert_eq!(body.kind_id(), "body");
        assert_eq!(body.identity(), "body:event:0000000000000004:00000002");
        assert_eq!(reservoir.kind_id(), "reservoir");
        assert_eq!(
            reservoir.identity(),
            "reservoir:event:0000000000000004:00000003"
        );
        assert_ne!(body.identity(), format!("{body:?}"));
        assert_ne!(reservoir.identity(), format!("{reservoir:?}"));
    }
}

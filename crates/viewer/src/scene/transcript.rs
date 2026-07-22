//! Exact transcript and floor-ingress projections.

/// Why value-level accounting cannot yet be rendered by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValuePayloadVisibility {
    /// The sealed transcript exposes the event kind but keeps its value record
    /// inside a planet-owned event variant that the viewer cannot name.
    OpaqueAtViewerBoundary,
}

/// Typed status for provenance inspection over the current receipt surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProvenanceScene {
    value_payload_visibility: ValuePayloadVisibility,
}

impl ProvenanceScene {
    /// Whether per-value tier and provenance payloads can be inspected.
    pub const fn value_payload_visibility(self) -> ValuePayloadVisibility {
        self.value_payload_visibility
    }
}

/// One exact SI representation value used only as an engine coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentationValueScene {
    kind_id: &'static str,
    symbol: &'static str,
    bits: i128,
    scale_bits: u32,
}

impl RepresentationValueScene {
    /// Whether this is an exact definition or a derived definition.
    pub const fn kind_id(self) -> &'static str {
        self.kind_id
    }

    /// Stable SI symbol.
    pub const fn symbol(self) -> &'static str {
        self.symbol
    }

    /// Exact signed integer payload.
    pub const fn bits(self) -> i128 {
        self.bits
    }

    /// Binary scale paired with [`Self::bits`].
    pub const fn scale_bits(self) -> u32 {
        self.scale_bits
    }
}

/// One transcript event in canonical append order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscriptEventScene {
    ordinal: u64,
    kind_id: &'static str,
}

impl TranscriptEventScene {
    /// Stable zero-based event ordinal minted by the transcript.
    pub const fn ordinal(self) -> u64 {
        self.ordinal
    }

    /// Stable event-kind identity.
    pub const fn kind_id(self) -> &'static str {
        self.kind_id
    }
}

/// One floor-ingress event whose value payload remains planet-owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloorEventScene {
    ordinal: u64,
    value_payload_visibility: ValuePayloadVisibility,
}

impl FloorEventScene {
    /// Stable transcript ordinal.
    pub const fn ordinal(self) -> u64 {
        self.ordinal
    }

    /// Explicit boundary on quantity, exact bits, tier, and provenance.
    pub const fn value_payload_visibility(self) -> ValuePayloadVisibility {
        self.value_payload_visibility
    }
}

/// Exact floor-ingress summary available at the observer boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorScene {
    declared_entry_count: usize,
    events: Vec<FloorEventScene>,
}

impl FloorScene {
    /// Floor-entry count sealed into the receipt.
    pub const fn declared_entry_count(&self) -> usize {
        self.declared_entry_count
    }

    /// Floor events in canonical transcript order.
    pub fn events(&self) -> &[FloorEventScene] {
        &self.events
    }

    /// Whether every declared floor entry has a visible ingress event.
    pub fn all_declared_entries_observed(&self) -> bool {
        self.declared_entry_count == self.events.len()
    }
}

/// Lazy immutable projection of the receipt's append-only transcript.
#[derive(Debug, Clone, Copy)]
pub struct TranscriptScene<'a> {
    receipt: &'a civsim_planet::RunReceipt,
}

impl<'a> TranscriptScene<'a> {
    pub(super) const fn new(receipt: &'a civsim_planet::RunReceipt) -> Self {
        Self { receipt }
    }

    /// Transcript schema identity.
    pub const fn schema_id(self) -> &'static str {
        self.receipt.transcript().schema().id()
    }

    /// Transcript schema major version.
    pub const fn schema_major(self) -> u16 {
        self.receipt.transcript().schema().major()
    }

    /// Transcript schema minor version.
    pub const fn schema_minor(self) -> u16 {
        self.receipt.transcript().schema().minor()
    }

    /// Whether completion or refusal closed the transcript.
    pub const fn is_closed(self) -> bool {
        self.receipt.transcript().is_closed()
    }

    /// Noncausal SI representation schema identity.
    pub const fn representation_schema_id(self) -> &'static str {
        self.receipt.transcript().representation().schema_id()
    }

    /// Exact SI representation values in their sealed order.
    pub fn representation_values(
        self,
    ) -> impl ExactSizeIterator<Item = RepresentationValueScene> + 'a {
        self.receipt
            .transcript()
            .representation()
            .values()
            .iter()
            .map(|record| {
                let value = record.value();
                RepresentationValueScene {
                    kind_id: record.kind_id(),
                    symbol: record.symbol(),
                    bits: value.bits(),
                    scale_bits: value.scale_bits(),
                }
            })
    }

    /// Events in exact canonical append order.
    pub fn events(self) -> impl ExactSizeIterator<Item = TranscriptEventScene> + 'a {
        self.receipt
            .transcript()
            .events()
            .iter()
            .map(|event| TranscriptEventScene {
                ordinal: event.id().ordinal(),
                kind_id: event.kind().id(),
            })
    }

    /// Floor ingress without fabricating opaque value payloads.
    pub fn floor(self) -> FloorScene {
        let events = self
            .events()
            .filter(|event| event.kind_id() == "floor_value")
            .map(|event| FloorEventScene {
                ordinal: event.ordinal(),
                value_payload_visibility: ValuePayloadVisibility::OpaqueAtViewerBoundary,
            })
            .collect();
        FloorScene {
            declared_entry_count: self.receipt.transcript().declared_floor_entries(),
            events,
        }
    }

    /// Report the current typed boundary on per-value provenance inspection.
    pub const fn provenance(self) -> ProvenanceScene {
        ProvenanceScene {
            value_payload_visibility: ValuePayloadVisibility::OpaqueAtViewerBoundary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_value_payloads_never_acquire_a_guessed_mark() {
        let provenance = ProvenanceScene {
            value_payload_visibility: ValuePayloadVisibility::OpaqueAtViewerBoundary,
        };
        assert_eq!(
            provenance.value_payload_visibility(),
            ValuePayloadVisibility::OpaqueAtViewerBoundary
        );
    }
}

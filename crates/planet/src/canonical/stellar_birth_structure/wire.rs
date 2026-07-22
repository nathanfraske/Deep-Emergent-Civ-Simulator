//! Canonical text encoding for the value-free stellar-birth structure.

use super::{validate_structure_schema, StellarBirthStructureSchema};
use crate::canonical::transcript::canonical_text;
use std::fmt;

pub(in crate::canonical) fn write_stellar_birth_structure(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    structure: &StellarBirthStructureSchema,
) -> fmt::Result {
    if validate_structure_schema(structure).is_err() {
        return Err(fmt::Error);
    }
    let prefix = format!("{prefix}.structure");
    writeln!(f, "{prefix}.schema={}", canonical_text(structure.schema_id))?;

    let component = &structure.component_registry;
    let component_prefix = format!("{prefix}.component_registry");
    writeln!(
        f,
        "{component_prefix}.schema={}",
        canonical_text(component.schema_id)
    )?;
    writeln!(
        f,
        "{component_prefix}.cardinality_rule={}",
        component.cardinality_rule.id()
    )?;
    writeln!(
        f,
        "{component_prefix}.identity_rule={}",
        component.identity_rule.id()
    )?;
    writeln!(
        f,
        "{component_prefix}.ordering_rule={}",
        component.ordering_rule.id()
    )?;
    writeln!(
        f,
        "{component_prefix}.topology_rule={}",
        component.topology_rule.id()
    )?;
    writeln!(
        f,
        "{component_prefix}.capacity_rule={}",
        component.capacity_rule.id()
    )?;
    writeln!(
        f,
        "{component_prefix}.ordinal_rule={}",
        component.ordinal_rule.id()
    )?;
    writeln!(
        f,
        "{component_prefix}.encoding_rule={}",
        component.encoding_rule.id()
    )?;
    writeln!(
        f,
        "{component_prefix}.collision_rule={}",
        component.collision_rule.id()
    )?;
    writeln!(
        f,
        "{component_prefix}.symmetry_rule={}",
        component.symmetry_rule.id()
    )?;
    writeln!(
        f,
        "{component_prefix}.topology_label_authority_rule={}",
        component.topology_label_authority_rule.id()
    )?;

    let species = &structure.species_registry;
    let species_prefix = format!("{prefix}.species_registry");
    writeln!(
        f,
        "{species_prefix}.schema={}",
        canonical_text(species.schema_id)
    )?;
    writeln!(
        f,
        "{species_prefix}.membership_rule={}",
        species.membership_rule.id()
    )?;
    writeln!(
        f,
        "{species_prefix}.identity_rule={}",
        species.identity_rule.id()
    )?;
    writeln!(
        f,
        "{species_prefix}.ordering_rule={}",
        species.ordering_rule.id()
    )?;
    writeln!(
        f,
        "{species_prefix}.capacity_rule={}",
        species.capacity_rule.id()
    )?;
    writeln!(
        f,
        "{species_prefix}.ordinal_rule={}",
        species.ordinal_rule.id()
    )?;

    writeln!(
        f,
        "{prefix}.index_domain_count={}",
        structure.index_domains.len()
    )?;
    for (index, domain) in structure.index_domains.iter().enumerate() {
        let domain_prefix = format!("{prefix}.index_domain.{index:04}");
        writeln!(f, "{domain_prefix}.id={}", canonical_text(domain.id))?;
        writeln!(f, "{domain_prefix}.kind={}", domain.kind.id())?;
        writeln!(
            f,
            "{domain_prefix}.support_rule={}",
            domain.support_rule.id()
        )?;
        writeln!(
            f,
            "{domain_prefix}.resolution_rule={}",
            domain.resolution_rule.id()
        )?;
        writeln!(
            f,
            "{domain_prefix}.ordering_rule={}",
            domain.ordering_rule.id()
        )?;
        writeln!(
            f,
            "{domain_prefix}.capacity_rule={}",
            domain.capacity_rule.id()
        )?;
        writeln!(
            f,
            "{domain_prefix}.ordinal_rule={}",
            domain.ordinal_rule.id()
        )?;
        writeln!(
            f,
            "{domain_prefix}.reference_rule={}",
            domain.reference_rule.id()
        )?;
        write_dimension(
            f,
            &format!("{domain_prefix}.coordinate"),
            domain.coordinate_dimension,
        )?;
    }

    writeln!(
        f,
        "{prefix}.carrier_count={}",
        structure.carrier_schemas.len()
    )?;
    for (index, carrier) in structure.carrier_schemas.iter().enumerate() {
        let carrier_prefix = format!("{prefix}.carrier.{index:04}");
        writeln!(f, "{carrier_prefix}.id={}", carrier.id())?;
        writeln!(
            f,
            "{carrier_prefix}.value_shape={}",
            carrier.value_shape.id()
        )?;
        writeln!(
            f,
            "{carrier_prefix}.index_domain_count={}",
            carrier.index_domain_ids.len()
        )?;
        for (domain_index, domain_id) in carrier.index_domain_ids.iter().enumerate() {
            writeln!(
                f,
                "{carrier_prefix}.index_domain.{domain_index:04}={}",
                canonical_text(domain_id)
            )?;
        }
        writeln!(
            f,
            "{carrier_prefix}.normalization={}",
            carrier.normalization.id()
        )?;
        writeln!(
            f,
            "{carrier_prefix}.measure_semantics={}",
            carrier.measure_semantics.id()
        )?;
        writeln!(
            f,
            "{carrier_prefix}.support_rule={}",
            carrier.support_rule.id()
        )?;
    }
    Ok(())
}

fn write_dimension(f: &mut fmt::Formatter<'_>, prefix: &str, dimension: [i8; 7]) -> fmt::Result {
    let [length, mass, time, current, temperature, amount, luminous_intensity] = dimension;
    writeln!(f, "{prefix}.dimension.length={length}")?;
    writeln!(f, "{prefix}.dimension.mass={mass}")?;
    writeln!(f, "{prefix}.dimension.time={time}")?;
    writeln!(f, "{prefix}.dimension.current={current}")?;
    writeln!(f, "{prefix}.dimension.temperature={temperature}")?;
    writeln!(f, "{prefix}.dimension.amount={amount}")?;
    writeln!(
        f,
        "{prefix}.dimension.luminous_intensity={luminous_intensity}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::stellar_birth_structure::stellar_birth_structure_schema;
    use std::fmt::Write;

    struct StructureDisplay<'a>(&'a StellarBirthStructureSchema);

    impl fmt::Display for StructureDisplay<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_stellar_birth_structure(f, "fixture", self.0)
        }
    }

    #[test]
    fn invalid_structure_cannot_serialize_under_the_v1_label() {
        let mut schema = stellar_birth_structure_schema().unwrap();
        schema.index_domains.swap(0, 1);
        let mut output = String::new();
        assert!(write!(&mut output, "{}", StructureDisplay(&schema)).is_err());
        assert!(output.is_empty());
    }
}

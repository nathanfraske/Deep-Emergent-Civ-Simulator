//! Exact species-state reduction behind the open stellar-birth refusal.
//!
//! The retained substrate computed a familiar gas weight from an element table,
//! one abundance convention, and a caller-selected molecular rule. None of
//! those inputs belong on the canonical path. This module keeps only the
//! invariant weighted reduction over a complete, physically derived support.
//!
//! No repository species support exists yet. The verifier is sealed behind an
//! authority token with no production constructor, so this code cannot invent
//! a familiar species, attach a cited mass, or close either Stage 1 proof. A
//! future authority must derive every content identity, rest mass, physical
//! state, support weight, and applicability proof from admitted dependencies.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "exact species-state kernel is sealed until its derivation authority lands"
    )
)]

use civsim_units::bignum::BigRat;
use std::{cmp::Ordering, fmt};

pub(super) const COMPLETE_SPECIES_STATE_MEAN_PARTICLE_MASS_LAW_ID: &str =
    "candidate.composition_weighted_particle_mass";

/// Content-derived physical identity. Names, classes, and serialization order
/// are not part of this identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SpeciesContentIdentity([u8; 32]);

/// Opaque proof that the candidate's charge, state, active sectors, validity
/// domain, dependency ancestry, and floor binding admit it on this support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DerivedSpeciesStateProof {
    _seal: SpeciesStateProofSeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpeciesStateProofSeal;

/// Candidate produced by the future species-state derivation authority.
/// Required fields stay optional until verification so absence has a typed
/// refusal instead of a default mass or familiar state.
#[derive(Debug, Clone)]
struct DerivedSpeciesStateCandidate {
    identity: SpeciesContentIdentity,
    rest_mass: Option<BigRat>,
    state_proof: Option<DerivedSpeciesStateProof>,
}

/// One exact number-fraction coordinate over the candidate support.
#[derive(Debug, Clone)]
struct SpeciesSupportWeight {
    identity: SpeciesContentIdentity,
    number_fraction: BigRat,
}

/// Authority needed to verify a realized registry. There is no production
/// constructor in this slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpeciesRegistryAuthority {
    _seal: SpeciesRegistryAuthoritySeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpeciesRegistryAuthoritySeal;

#[derive(Debug, Clone)]
struct VerifiedSpeciesStateEntry {
    identity: SpeciesContentIdentity,
    rest_mass: BigRat,
    number_fraction: BigRat,
}

/// Canonically ordered, complete support whose exact number fractions sum to
/// one without renormalization.
#[derive(Debug, Clone)]
pub(super) struct VerifiedSpeciesStateSupport {
    entries: Vec<VerifiedSpeciesStateEntry>,
    _authority: SpeciesRegistryAuthority,
}

/// Exact derived result. This is an internal rational law result, not a
/// production bitstream representation. A later quantity schema must choose
/// and receipt the deterministic integer projection before the value can be
/// written as physical state.
#[derive(Debug, Clone)]
pub(super) struct DerivedMeanParticleMass {
    value: BigRat,
    support_identities: Vec<SpeciesContentIdentity>,
}

impl DerivedMeanParticleMass {
    pub(super) const fn law_id(&self) -> &'static str {
        COMPLETE_SPECIES_STATE_MEAN_PARTICLE_MASS_LAW_ID
    }

    pub(super) fn exact_value(&self) -> &BigRat {
        &self.value
    }

    pub(super) fn support_identities(&self) -> &[SpeciesContentIdentity] {
        &self.support_identities
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SpeciesStateSupportRefusal {
    EmptySupport,
    MissingRestMass(SpeciesContentIdentity),
    NegativeRestMass(SpeciesContentIdentity),
    UnverifiedPhysicalState(SpeciesContentIdentity),
    DuplicateContentIdentity(SpeciesContentIdentity),
    ContentIdentityCollision(SpeciesContentIdentity),
    DuplicateSupportWeight(SpeciesContentIdentity),
    UnknownSupportMember(SpeciesContentIdentity),
    MissingSupportMember(SpeciesContentIdentity),
    NonPositiveSupportWeight(SpeciesContentIdentity),
    NonUnitCompositionSimplex,
}

impl fmt::Display for SpeciesStateSupportRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySupport => f.write_str("stellar-birth species-state support is empty"),
            Self::MissingRestMass(identity) => {
                write!(f, "species-state {} has no derived rest mass", identity)
            }
            Self::NegativeRestMass(identity) => {
                write!(f, "species-state {} has a negative rest mass", identity)
            }
            Self::UnverifiedPhysicalState(identity) => write!(
                f,
                "species-state {} has no admitted state and applicability proof",
                identity
            ),
            Self::DuplicateContentIdentity(identity) => {
                write!(f, "species-state {} is duplicated", identity)
            }
            Self::ContentIdentityCollision(identity) => write!(
                f,
                "species-state {} identifies unequal physical content",
                identity
            ),
            Self::DuplicateSupportWeight(identity) => {
                write!(
                    f,
                    "species-state {} has duplicate support weights",
                    identity
                )
            }
            Self::UnknownSupportMember(identity) => {
                write!(f, "species-state {} is weighted but not derived", identity)
            }
            Self::MissingSupportMember(identity) => {
                write!(
                    f,
                    "species-state {} is derived but absent from support",
                    identity
                )
            }
            Self::NonPositiveSupportWeight(identity) => {
                write!(
                    f,
                    "species-state {} has a non-positive support weight",
                    identity
                )
            }
            Self::NonUnitCompositionSimplex => {
                f.write_str("species-state number fractions do not sum exactly to one")
            }
        }
    }
}

impl fmt::Display for SpeciesContentIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0[..6] {
            write!(f, "{byte:02x}")?;
        }
        f.write_str("...")
    }
}

fn strictly_positive(value: &BigRat) -> bool {
    value.cmp_rat(&BigRat::from_i64(0)) == Ordering::Greater
}

fn nonnegative(value: &BigRat) -> bool {
    value.cmp_rat(&BigRat::from_i64(0)) != Ordering::Less
}

fn same_candidate_content(
    left: &DerivedSpeciesStateCandidate,
    right: &DerivedSpeciesStateCandidate,
) -> bool {
    match (&left.rest_mass, &right.rest_mass) {
        (Some(left_mass), Some(right_mass)) => {
            left_mass.cmp_rat(right_mass) == Ordering::Equal
                && left.state_proof == right.state_proof
        }
        (None, None) => left.state_proof == right.state_proof,
        _ => false,
    }
}

fn verify_species_state_support(
    authority: SpeciesRegistryAuthority,
    mut candidates: Vec<DerivedSpeciesStateCandidate>,
    mut weights: Vec<SpeciesSupportWeight>,
) -> Result<VerifiedSpeciesStateSupport, SpeciesStateSupportRefusal> {
    if candidates.is_empty() {
        return Err(SpeciesStateSupportRefusal::EmptySupport);
    }

    candidates.sort_by_key(|candidate| candidate.identity);
    let mut group_start = 0;
    while group_start < candidates.len() {
        let identity = candidates[group_start].identity;
        let mut group_end = group_start + 1;
        while group_end < candidates.len() && candidates[group_end].identity == identity {
            group_end += 1;
        }
        if group_end - group_start > 1 {
            let first = &candidates[group_start];
            let has_unequal_content = candidates[group_start + 1..group_end]
                .iter()
                .any(|candidate| !same_candidate_content(first, candidate));
            return Err(if has_unequal_content {
                SpeciesStateSupportRefusal::ContentIdentityCollision(identity)
            } else {
                SpeciesStateSupportRefusal::DuplicateContentIdentity(identity)
            });
        }
        group_start = group_end;
    }

    for candidate in &candidates {
        let Some(rest_mass) = &candidate.rest_mass else {
            return Err(SpeciesStateSupportRefusal::MissingRestMass(
                candidate.identity,
            ));
        };
        if !nonnegative(rest_mass) {
            return Err(SpeciesStateSupportRefusal::NegativeRestMass(
                candidate.identity,
            ));
        }
        if candidate.state_proof.is_none() {
            return Err(SpeciesStateSupportRefusal::UnverifiedPhysicalState(
                candidate.identity,
            ));
        }
    }

    weights.sort_by_key(|weight| weight.identity);
    for pair in weights.windows(2) {
        if pair[0].identity == pair[1].identity {
            return Err(SpeciesStateSupportRefusal::DuplicateSupportWeight(
                pair[0].identity,
            ));
        }
    }
    for weight in &weights {
        if !strictly_positive(&weight.number_fraction) {
            return Err(SpeciesStateSupportRefusal::NonPositiveSupportWeight(
                weight.identity,
            ));
        }
        if candidates
            .binary_search_by_key(&weight.identity, |candidate| candidate.identity)
            .is_err()
        {
            return Err(SpeciesStateSupportRefusal::UnknownSupportMember(
                weight.identity,
            ));
        }
    }
    for candidate in &candidates {
        if weights
            .binary_search_by_key(&candidate.identity, |weight| weight.identity)
            .is_err()
        {
            return Err(SpeciesStateSupportRefusal::MissingSupportMember(
                candidate.identity,
            ));
        }
    }

    let total_weight = weights.iter().fold(BigRat::from_i64(0), |sum, weight| {
        sum.add(&weight.number_fraction).reduce()
    });
    if total_weight.cmp_rat(&BigRat::from_i64(1)) != Ordering::Equal {
        return Err(SpeciesStateSupportRefusal::NonUnitCompositionSimplex);
    }

    let entries = candidates
        .into_iter()
        .zip(weights)
        .map(|(candidate, weight)| {
            if candidate.identity != weight.identity {
                return Err(if candidate.identity < weight.identity {
                    SpeciesStateSupportRefusal::MissingSupportMember(candidate.identity)
                } else {
                    SpeciesStateSupportRefusal::UnknownSupportMember(weight.identity)
                });
            }
            let rest_mass =
                candidate
                    .rest_mass
                    .ok_or(SpeciesStateSupportRefusal::MissingRestMass(
                        candidate.identity,
                    ))?;
            Ok(VerifiedSpeciesStateEntry {
                identity: candidate.identity,
                rest_mass,
                number_fraction: weight.number_fraction,
            })
        })
        .collect::<Result<Vec<_>, SpeciesStateSupportRefusal>>()?;

    Ok(VerifiedSpeciesStateSupport {
        entries,
        _authority: authority,
    })
}

/// Reduce a verified support to the exact mean free-particle mass.
///
/// Every molecular, ionized, neutral, unfamiliar, or additional-sector state
/// is already a distinct content identity at this boundary. The reducer has no
/// named-species branch and no phase or particle-count selector.
pub(super) fn derive_mean_particle_mass(
    support: &VerifiedSpeciesStateSupport,
) -> DerivedMeanParticleMass {
    let mut weighted_mass = BigRat::from_i64(0);
    let mut total_weight = BigRat::from_i64(0);
    let mut support_identities = Vec::with_capacity(support.entries.len());
    for entry in &support.entries {
        weighted_mass = weighted_mass
            .add(&entry.number_fraction.mul(&entry.rest_mass))
            .reduce();
        total_weight = total_weight.add(&entry.number_fraction).reduce();
        support_identities.push(entry.identity);
    }
    DerivedMeanParticleMass {
        value: weighted_mass.div(&total_weight).reduce(),
        support_identities,
    }
}

/// The repository has not yet derived a complete species-state support.
pub(super) const fn resolve_repository_species_state_support() -> Option<VerifiedSpeciesStateSupport>
{
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(tag: u8) -> SpeciesContentIdentity {
        let mut bytes = [0_u8; 32];
        bytes[0] = tag;
        SpeciesContentIdentity(bytes)
    }

    fn ratio(numerator: i64, denominator: i64) -> BigRat {
        BigRat::from_i64(numerator).div(&BigRat::from_i64(denominator))
    }

    fn state(tag: u8, rest_mass: Option<BigRat>) -> DerivedSpeciesStateCandidate {
        DerivedSpeciesStateCandidate {
            identity: identity(tag),
            rest_mass,
            state_proof: Some(DerivedSpeciesStateProof {
                _seal: SpeciesStateProofSeal,
            }),
        }
    }

    fn weight(tag: u8, number_fraction: BigRat) -> SpeciesSupportWeight {
        SpeciesSupportWeight {
            identity: identity(tag),
            number_fraction,
        }
    }

    fn authority() -> SpeciesRegistryAuthority {
        SpeciesRegistryAuthority {
            _seal: SpeciesRegistryAuthoritySeal,
        }
    }

    fn verify(
        candidates: Vec<DerivedSpeciesStateCandidate>,
        weights: Vec<SpeciesSupportWeight>,
    ) -> Result<VerifiedSpeciesStateSupport, SpeciesStateSupportRefusal> {
        verify_species_state_support(authority(), candidates, weights)
    }

    #[test]
    fn exact_weighted_mean_is_identity_neutral_and_permutation_invariant() {
        let candidates = vec![state(1, Some(ratio(2, 1))), state(2, Some(ratio(6, 1)))];
        let weights = vec![weight(1, ratio(1, 4)), weight(2, ratio(3, 4))];
        let forward = verify(candidates.clone(), weights.clone()).unwrap();

        let mut reversed_candidates = candidates;
        reversed_candidates.reverse();
        let mut reversed_weights = weights;
        reversed_weights.reverse();
        let reversed = verify(reversed_candidates, reversed_weights).unwrap();

        let forward_mean = derive_mean_particle_mass(&forward);
        let reversed_mean = derive_mean_particle_mass(&reversed);
        assert_eq!(
            forward_mean.exact_value().cmp_rat(&BigRat::from_i64(5)),
            Ordering::Equal
        );
        assert_eq!(
            forward_mean
                .exact_value()
                .cmp_rat(reversed_mean.exact_value()),
            Ordering::Equal
        );
        assert_eq!(
            forward_mean.support_identities(),
            reversed_mean.support_identities()
        );
        assert_eq!(
            forward_mean.law_id(),
            COMPLETE_SPECIES_STATE_MEAN_PARTICLE_MASS_LAW_ID
        );
    }

    #[test]
    fn unfamiliar_state_is_accepted_by_proof_not_by_class_dispatch() {
        let support = verify(
            vec![state(241, Some(ratio(7, 3)))],
            vec![weight(241, BigRat::from_i64(1))],
        )
        .unwrap();
        assert_eq!(
            derive_mean_particle_mass(&support)
                .exact_value()
                .cmp_rat(&ratio(7, 3)),
            Ordering::Equal
        );
    }

    #[test]
    fn duplicate_identity_and_content_collision_refuse() {
        let duplicate = state(1, Some(BigRat::from_i64(2)));
        assert_eq!(
            verify(
                vec![duplicate.clone(), duplicate],
                vec![weight(1, BigRat::from_i64(1))]
            )
            .unwrap_err(),
            SpeciesStateSupportRefusal::DuplicateContentIdentity(identity(1))
        );
        assert_eq!(
            verify(
                vec![
                    state(1, Some(BigRat::from_i64(2))),
                    state(1, Some(BigRat::from_i64(3)))
                ],
                vec![weight(1, BigRat::from_i64(1))]
            )
            .unwrap_err(),
            SpeciesStateSupportRefusal::ContentIdentityCollision(identity(1))
        );

        let content_a = state(1, Some(BigRat::from_i64(2)));
        let content_b = state(1, Some(BigRat::from_i64(3)));
        for candidates in [
            vec![content_a.clone(), content_a.clone(), content_b.clone()],
            vec![content_a.clone(), content_b.clone(), content_a.clone()],
            vec![content_b, content_a.clone(), content_a],
        ] {
            assert_eq!(
                verify(candidates, vec![weight(1, BigRat::from_i64(1))]).unwrap_err(),
                SpeciesStateSupportRefusal::ContentIdentityCollision(identity(1))
            );
        }
    }

    #[test]
    fn unknown_missing_and_duplicate_support_members_refuse() {
        assert_eq!(
            verify(
                vec![state(1, Some(BigRat::from_i64(2)))],
                vec![weight(2, BigRat::from_i64(1))]
            )
            .unwrap_err(),
            SpeciesStateSupportRefusal::UnknownSupportMember(identity(2))
        );
        assert_eq!(
            verify(
                vec![
                    state(1, Some(BigRat::from_i64(2))),
                    state(2, Some(BigRat::from_i64(3)))
                ],
                vec![weight(1, BigRat::from_i64(1))]
            )
            .unwrap_err(),
            SpeciesStateSupportRefusal::MissingSupportMember(identity(2))
        );
        assert_eq!(
            verify(
                vec![state(1, Some(BigRat::from_i64(2)))],
                vec![weight(1, ratio(1, 2)), weight(1, ratio(1, 2))]
            )
            .unwrap_err(),
            SpeciesStateSupportRefusal::DuplicateSupportWeight(identity(1))
        );
    }

    #[test]
    fn incomplete_mass_state_and_simplex_refuse_without_defaults() {
        assert_eq!(
            verify(Vec::new(), Vec::new()).unwrap_err(),
            SpeciesStateSupportRefusal::EmptySupport
        );
        assert_eq!(
            verify(vec![state(1, None)], vec![weight(1, BigRat::from_i64(1))]).unwrap_err(),
            SpeciesStateSupportRefusal::MissingRestMass(identity(1))
        );
        assert_eq!(
            verify(
                vec![state(1, Some(BigRat::from_i64(-2)))],
                vec![weight(1, BigRat::from_i64(1))]
            )
            .unwrap_err(),
            SpeciesStateSupportRefusal::NegativeRestMass(identity(1))
        );

        let massless = verify(
            vec![state(1, Some(BigRat::from_i64(0)))],
            vec![weight(1, BigRat::from_i64(1))],
        )
        .unwrap();
        assert!(derive_mean_particle_mass(&massless).exact_value().is_zero());

        let mut unverified = state(1, Some(BigRat::from_i64(2)));
        unverified.state_proof = None;
        assert_eq!(
            verify(vec![unverified], vec![weight(1, BigRat::from_i64(1))]).unwrap_err(),
            SpeciesStateSupportRefusal::UnverifiedPhysicalState(identity(1))
        );

        assert_eq!(
            verify(
                vec![state(1, Some(BigRat::from_i64(2)))],
                vec![weight(1, ratio(3, 4))]
            )
            .unwrap_err(),
            SpeciesStateSupportRefusal::NonUnitCompositionSimplex
        );
        assert_eq!(
            verify(
                vec![state(1, Some(BigRat::from_i64(2)))],
                vec![weight(1, BigRat::from_i64(0))]
            )
            .unwrap_err(),
            SpeciesStateSupportRefusal::NonPositiveSupportWeight(identity(1))
        );
    }

    #[test]
    fn production_has_no_species_support_or_familiar_value_surface() {
        assert!(resolve_repository_species_state_support().is_none());
        let production = include_str!("stellar_birth_species.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for forbidden in [
            "SolarAbundances",
            "PeriodicTable",
            "hydrogen_atoms_per_molecule",
        ] {
            assert!(
                !production.contains(forbidden),
                "production species reducer contains forbidden surface {forbidden}"
            );
        }
    }
}

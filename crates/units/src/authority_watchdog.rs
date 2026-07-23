//! Independent mechanical cross-checks for authority-bearing units claims.
//!
//! A producer may share canonical input bytes, exact integer primitives, and
//! canonical serialization with its cross-checker. It may not supply the
//! semantic expected answer. The first enrolled claim is the per-phenomenon
//! Buckingham Pi budget used by absolute-floor admission.

use crate::{
    dimensional_analysis::{DimensionAnalysisError, SiDimensionAnalysis, SiDimensionColumn},
    fundamentals::{execution_root, SI_BASE_DIMENSION_IDS},
};
use civsim_ledger::DerivationExhaustionReceipt;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const PI_WATCHDOG_SCHEMA_ID: &str = "civsim.units.pi-budget-watchdog.v1";
const PI_PRODUCER_ID: &str = "civsim.units.exact-rational-rref.v1";
const PI_CROSS_CHECKER_ID: &str = "civsim.units.fraction-free-rank.v1";

#[derive(Debug, Clone, Copy)]
struct PhenomenonSpec {
    id: &'static str,
    ordered_variables: &'static [&'static str],
}

const ELECTROMAGNETIC_VARIABLES: [&str; 4] = ["c", "h", "e", "alpha"];
const GRAVITY_MATTER_VARIABLES: [&str; 5] = ["G", "m_e", "h", "c", "Delta_nu_Cs"];
const PHENOMENA: [PhenomenonSpec; 2] = [
    PhenomenonSpec {
        id: "electromagnetic.coupling",
        ordered_variables: &ELECTROMAGNETIC_VARIABLES,
    },
    PhenomenonSpec {
        id: "gravity_matter.scale_basis",
        ordered_variables: &GRAVITY_MATTER_VARIABLES,
    },
];

#[derive(Debug, Clone, Copy)]
struct ExpectedPiReceipt {
    phenomenon: &'static str,
    sha256: [u8; 32],
}

// These pins are updated only after independent review of the declared
// phenomenon variable set. Algebra can prove rank over that set, but cannot
// prove that the set itself is physically complete.
const EXPECTED_PI_RECEIPTS: [ExpectedPiReceipt; 2] = [
    ExpectedPiReceipt {
        phenomenon: "electromagnetic.coupling",
        sha256: [
            0x99, 0x57, 0xd4, 0x6f, 0x3c, 0x8c, 0x28, 0xdb, 0x3e, 0x25, 0x36, 0xf4, 0x1c, 0xf4,
            0x38, 0xda, 0x34, 0x70, 0xce, 0xa4, 0xe5, 0x01, 0xf1, 0x78, 0xef, 0xb9, 0x4f, 0xc1,
            0x0b, 0x47, 0x88, 0x26,
        ],
    },
    ExpectedPiReceipt {
        phenomenon: "gravity_matter.scale_basis",
        sha256: [
            0x56, 0xcf, 0x61, 0x71, 0xbf, 0xab, 0x28, 0x38, 0xeb, 0x5b, 0x4a, 0xc9, 0x1c, 0x05,
            0xf9, 0x68, 0xdd, 0x0f, 0xd2, 0x8c, 0xc8, 0x2e, 0xc7, 0xb0, 0x8c, 0x9b, 0x64, 0x9c,
            0xb4, 0x6b, 0x66, 0xc8,
        ],
    },
];

/// Sealed agreement of the Pi producer and its independent cross-checker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PiBudgetWatchdogSeal {
    digest: [u8; 32],
}

impl PiBudgetWatchdogSeal {
    pub(crate) const fn digest(self) -> [u8; 32] {
        self.digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PiBudgetWatchdogReceipt {
    phenomenon: String,
    matrix_digest: [u8; 32],
    producer_rank: usize,
    checker_rank: usize,
    nullity: usize,
    declared_budget: usize,
    residual_slots: Vec<String>,
    producer_basis_digest: [u8; 32],
}

/// Failures of the independent Pi budget check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityWatchdogError {
    UnknownPhenomenon(String),
    MissingPhenomenon(String),
    MissingVariable {
        phenomenon: String,
        variable: String,
    },
    DuplicateResidualSlot {
        phenomenon: String,
        slot: String,
    },
    EmptyResidualSlot(String),
    ResidualSlotCount {
        phenomenon: String,
        computed: usize,
        declared: usize,
    },
    InconsistentDeclaredBudget {
        phenomenon: String,
    },
    Producer(DimensionAnalysisError),
    CheckerArithmeticOverflow,
    RankDisagreement {
        phenomenon: String,
        producer: usize,
        checker: usize,
    },
    InvalidProducerBasis(String),
    BudgetDisagreement {
        phenomenon: String,
        computed: usize,
        declared: usize,
    },
    ReceiptSealMismatch {
        phenomenon: String,
        expected: String,
        found: String,
    },
}

impl fmt::Display for AuthorityWatchdogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPhenomenon(value) => {
                write!(f, "Pi watchdog has no variable-set authority for '{value}'")
            }
            Self::MissingPhenomenon(value) => {
                write!(f, "Pi watchdog received no receipt for '{value}'")
            }
            Self::MissingVariable {
                phenomenon,
                variable,
            } => write!(
                f,
                "Pi watchdog phenomenon '{phenomenon}' names absent variable '{variable}'"
            ),
            Self::DuplicateResidualSlot { phenomenon, slot } => write!(
                f,
                "Pi watchdog phenomenon '{phenomenon}' repeats residual slot '{slot}'"
            ),
            Self::EmptyResidualSlot(phenomenon) => write!(
                f,
                "Pi watchdog phenomenon '{phenomenon}' contains an empty residual slot"
            ),
            Self::ResidualSlotCount {
                phenomenon,
                computed,
                declared,
            } => write!(
                f,
                "Pi residual-slot disagreement for '{phenomenon}': nullity {computed}, slots {declared}"
            ),
            Self::InconsistentDeclaredBudget { phenomenon } => write!(
                f,
                "Pi watchdog phenomenon '{phenomenon}' carries inconsistent declared budgets"
            ),
            Self::Producer(error) => write!(f, "Pi producer failed: {error}"),
            Self::CheckerArithmeticOverflow => {
                f.write_str("Pi cross-checker exceeded its checked i128 domain")
            }
            Self::RankDisagreement {
                phenomenon,
                producer,
                checker,
            } => write!(
                f,
                "Pi rank disagreement for '{phenomenon}': producer {producer}, checker {checker}"
            ),
            Self::InvalidProducerBasis(phenomenon) => write!(
                f,
                "Pi producer emitted a non-null basis vector for '{phenomenon}'"
            ),
            Self::BudgetDisagreement {
                phenomenon,
                computed,
                declared,
            } => write!(
                f,
                "Pi budget disagreement for '{phenomenon}': computed {computed}, declared {declared}"
            ),
            Self::ReceiptSealMismatch {
                phenomenon,
                expected,
                found,
            } => write!(
                f,
                "Pi watchdog receipt '{phenomenon}' expected {expected} but found {found}"
            ),
        }
    }
}

impl std::error::Error for AuthorityWatchdogError {}

impl From<DimensionAnalysisError> for AuthorityWatchdogError {
    fn from(error: DimensionAnalysisError) -> Self {
        Self::Producer(error)
    }
}

/// Independently verify every floor receipt's per-phenomenon Pi budget.
pub(crate) fn verify_floor_pi_budgets(
    receipts: &[DerivationExhaustionReceipt],
) -> Result<PiBudgetWatchdogSeal, AuthorityWatchdogError> {
    verify_with_specs(receipts, &PHENOMENA, &EXPECTED_PI_RECEIPTS)
}

fn verify_with_specs(
    receipts: &[DerivationExhaustionReceipt],
    specs: &[PhenomenonSpec],
    expected_receipts: &[ExpectedPiReceipt],
) -> Result<PiBudgetWatchdogSeal, AuthorityWatchdogError> {
    let mut grouped: BTreeMap<String, Vec<&DerivationExhaustionReceipt>> = BTreeMap::new();
    for receipt in receipts {
        if !specs.iter().any(|spec| spec.id == receipt.phenomenon) {
            return Err(AuthorityWatchdogError::UnknownPhenomenon(
                receipt.phenomenon.clone(),
            ));
        }
        grouped
            .entry(receipt.phenomenon.clone())
            .or_default()
            .push(receipt);
    }

    let mut receipt_digests = Vec::with_capacity(specs.len());
    for spec in specs {
        let group = grouped
            .get(spec.id)
            .ok_or_else(|| AuthorityWatchdogError::MissingPhenomenon(spec.id.to_owned()))?;
        let columns = columns_for(spec)?;
        let producer = SiDimensionAnalysis::analyze(&columns)?;
        let checker_rank = fraction_free_rank(&columns)?;
        if producer.rank() != checker_rank {
            return Err(AuthorityWatchdogError::RankDisagreement {
                phenomenon: spec.id.to_owned(),
                producer: producer.rank(),
                checker: checker_rank,
            });
        }
        let nullity = columns.len() - checker_rank;
        cross_check_basis(spec.id, &columns, producer.null_space_basis(), nullity)?;
        let declared_budget = group[0].buckingham_pi_groups;
        if group
            .iter()
            .any(|receipt| receipt.buckingham_pi_groups != declared_budget)
        {
            return Err(AuthorityWatchdogError::InconsistentDeclaredBudget {
                phenomenon: spec.id.to_owned(),
            });
        }
        if declared_budget != nullity {
            return Err(AuthorityWatchdogError::BudgetDisagreement {
                phenomenon: spec.id.to_owned(),
                computed: nullity,
                declared: declared_budget,
            });
        }

        let mut residual_slots = group
            .iter()
            .map(|receipt| receipt.residual_slot.trim().to_owned())
            .collect::<Vec<_>>();
        if residual_slots.iter().any(String::is_empty) {
            return Err(AuthorityWatchdogError::EmptyResidualSlot(
                spec.id.to_owned(),
            ));
        }
        if residual_slots.len() != nullity {
            return Err(AuthorityWatchdogError::ResidualSlotCount {
                phenomenon: spec.id.to_owned(),
                computed: nullity,
                declared: residual_slots.len(),
            });
        }
        residual_slots.sort();
        for pair in residual_slots.windows(2) {
            if pair[0] == pair[1] {
                return Err(AuthorityWatchdogError::DuplicateResidualSlot {
                    phenomenon: spec.id.to_owned(),
                    slot: pair[0].clone(),
                });
            }
        }

        let receipt = PiBudgetWatchdogReceipt {
            phenomenon: spec.id.to_owned(),
            matrix_digest: matrix_digest(spec, &columns),
            producer_rank: producer.rank(),
            checker_rank,
            nullity,
            declared_budget,
            residual_slots,
            producer_basis_digest: basis_digest(producer.null_space_basis()),
        };
        let digest = watchdog_receipt_digest(&receipt);
        let expected = expected_receipts
            .iter()
            .find(|expected| expected.phenomenon == spec.id)
            .ok_or_else(|| AuthorityWatchdogError::ReceiptSealMismatch {
                phenomenon: spec.id.to_owned(),
                expected: "missing expected receipt authority".to_owned(),
                found: hex(&digest),
            })?;
        if digest != expected.sha256 {
            return Err(AuthorityWatchdogError::ReceiptSealMismatch {
                phenomenon: spec.id.to_owned(),
                expected: hex(&expected.sha256),
                found: hex(&digest),
            });
        }
        receipt_digests.push(digest);
    }

    let mut seal = Sha256::new();
    hash_text(&mut seal, PI_WATCHDOG_SCHEMA_ID);
    for digest in receipt_digests {
        seal.update(digest);
    }
    Ok(PiBudgetWatchdogSeal {
        digest: seal.finalize().into(),
    })
}

fn columns_for(spec: &PhenomenonSpec) -> Result<Vec<SiDimensionColumn>, AuthorityWatchdogError> {
    spec.ordered_variables
        .iter()
        .map(|variable| {
            execution_root(variable)
                .map(|root| SiDimensionColumn::new(variable, root.dimension))
                .ok_or_else(|| AuthorityWatchdogError::MissingVariable {
                    phenomenon: spec.id.to_owned(),
                    variable: (*variable).to_owned(),
                })
        })
        .collect()
}

/// Fraction-free Gaussian rank over the integer SI exponent matrix.
///
/// This path shares matrix bytes with the producer, but shares no elimination,
/// pivot, null-space, expected-budget, or decision helper with rational RREF.
fn fraction_free_rank(columns: &[SiDimensionColumn]) -> Result<usize, AuthorityWatchdogError> {
    let rows = (0..SI_BASE_DIMENSION_IDS.len())
        .map(|base| {
            columns
                .iter()
                .map(|column| i128::from(column.dimension().exponents()[base]))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    fraction_free_row_rank(rows, columns.len())
}

fn fraction_free_row_rank(
    mut rows: Vec<Vec<i128>>,
    width: usize,
) -> Result<usize, AuthorityWatchdogError> {
    if rows.iter().any(|row| row.len() != width) {
        return Err(AuthorityWatchdogError::CheckerArithmeticOverflow);
    }
    let mut pivot_row = 0_usize;
    for column in 0..width {
        let Some(found) = (pivot_row..rows.len()).find(|row| rows[*row][column] != 0) else {
            continue;
        };
        rows.swap(pivot_row, found);
        let pivot = rows[pivot_row][column];
        let pivot_values = rows[pivot_row].clone();
        for row in rows.iter_mut().skip(pivot_row + 1) {
            let factor = row[column];
            if factor == 0 {
                continue;
            }
            for next in (column + 1)..width {
                row[next] = row[next]
                    .checked_mul(pivot)
                    .and_then(|left| {
                        pivot_values[next]
                            .checked_mul(factor)
                            .and_then(|right| left.checked_sub(right))
                    })
                    .ok_or(AuthorityWatchdogError::CheckerArithmeticOverflow)?;
            }
            row[column] = 0;
            reduce_integer_row(&mut row[(column + 1)..])?;
        }
        pivot_row += 1;
        if pivot_row == rows.len() {
            break;
        }
    }
    Ok(pivot_row)
}

fn reduce_integer_row(row: &mut [i128]) -> Result<(), AuthorityWatchdogError> {
    let divisor = row.iter().fold(0_u128, |current, value| {
        gcd_u128(current, value.unsigned_abs())
    });
    if divisor > 1 {
        for value in row {
            let negative = value.is_negative();
            let quotient = value.unsigned_abs() / divisor;
            *value = if negative {
                if quotient == 1_u128 << 127 {
                    i128::MIN
                } else {
                    -i128::try_from(quotient)
                        .map_err(|_| AuthorityWatchdogError::CheckerArithmeticOverflow)?
                }
            } else {
                i128::try_from(quotient)
                    .map_err(|_| AuthorityWatchdogError::CheckerArithmeticOverflow)?
            };
        }
    }
    Ok(())
}

fn cross_check_basis(
    phenomenon: &str,
    columns: &[SiDimensionColumn],
    basis: &[Vec<i128>],
    nullity: usize,
) -> Result<(), AuthorityWatchdogError> {
    if basis.len() != nullity {
        return Err(AuthorityWatchdogError::InvalidProducerBasis(
            phenomenon.to_owned(),
        ));
    }
    for vector in basis {
        if vector.len() != columns.len() || vector.iter().all(|coefficient| *coefficient == 0) {
            return Err(AuthorityWatchdogError::InvalidProducerBasis(
                phenomenon.to_owned(),
            ));
        }
        for base in 0..SI_BASE_DIMENSION_IDS.len() {
            let mut total = 0_i128;
            for (coefficient, column) in vector.iter().zip(columns) {
                total = total
                    .checked_add(
                        coefficient
                            .checked_mul(i128::from(column.dimension().exponents()[base]))
                            .ok_or(AuthorityWatchdogError::CheckerArithmeticOverflow)?,
                    )
                    .ok_or(AuthorityWatchdogError::CheckerArithmeticOverflow)?;
            }
            if total != 0 {
                return Err(AuthorityWatchdogError::InvalidProducerBasis(
                    phenomenon.to_owned(),
                ));
            }
        }
    }
    if fraction_free_row_rank(basis.to_vec(), columns.len())? != nullity {
        return Err(AuthorityWatchdogError::InvalidProducerBasis(
            phenomenon.to_owned(),
        ));
    }
    Ok(())
}

fn matrix_digest(spec: &PhenomenonSpec, columns: &[SiDimensionColumn]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash_text(&mut hash, "civsim.units.pi-matrix.v1");
    hash_text(&mut hash, spec.id);
    hash_usize(&mut hash, SI_BASE_DIMENSION_IDS.len());
    for base in SI_BASE_DIMENSION_IDS {
        hash_text(&mut hash, base);
    }
    hash_usize(&mut hash, columns.len());
    for column in columns {
        hash_text(&mut hash, column.id());
        for exponent in column.dimension().exponents() {
            hash.update(exponent.to_le_bytes());
        }
    }
    hash.finalize().into()
}

fn basis_digest(basis: &[Vec<i128>]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash_text(&mut hash, "civsim.units.pi-producer-basis.v1");
    hash_usize(&mut hash, basis.len());
    for vector in basis {
        hash_usize(&mut hash, vector.len());
        for coefficient in vector {
            hash.update(coefficient.to_le_bytes());
        }
    }
    hash.finalize().into()
}

fn watchdog_receipt_digest(receipt: &PiBudgetWatchdogReceipt) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash_text(&mut hash, PI_WATCHDOG_SCHEMA_ID);
    hash_text(&mut hash, PI_PRODUCER_ID);
    hash_text(&mut hash, PI_CROSS_CHECKER_ID);
    hash_text(&mut hash, &receipt.phenomenon);
    hash.update(receipt.matrix_digest);
    hash_usize(&mut hash, receipt.producer_rank);
    hash_usize(&mut hash, receipt.checker_rank);
    hash_usize(&mut hash, receipt.nullity);
    hash_usize(&mut hash, receipt.declared_budget);
    hash_usize(&mut hash, receipt.residual_slots.len());
    for slot in &receipt.residual_slots {
        hash_text(&mut hash, slot);
    }
    hash.update(receipt.producer_basis_digest);
    hash.finalize().into()
}

fn hash_text(hash: &mut Sha256, value: &str) {
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value.as_bytes());
}

fn hash_usize(hash: &mut Sha256, value: usize) {
    hash.update((value as u64).to_le_bytes());
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics_floor::physical_invariant_receipts;

    #[test]
    fn fraction_free_checker_reproduces_known_ranks_without_rref() {
        for spec in PHENOMENA {
            let columns = columns_for(&spec).unwrap();
            let producer = SiDimensionAnalysis::analyze(&columns).unwrap();
            let rank = fraction_free_rank(&columns).unwrap();
            assert_eq!(rank, producer.rank());
            cross_check_basis(
                spec.id,
                &columns,
                producer.null_space_basis(),
                columns.len() - rank,
            )
            .unwrap();
        }
    }

    #[test]
    fn basis_checker_rejects_zero_duplicate_and_incomplete_material() {
        let spec = PHENOMENA[1];
        let columns = columns_for(&spec).unwrap();
        let producer = SiDimensionAnalysis::analyze(&columns).unwrap();
        let nullity = columns.len() - fraction_free_rank(&columns).unwrap();
        let mut zero = producer.null_space_basis().to_vec();
        zero[0].fill(0);
        assert!(cross_check_basis(spec.id, &columns, &zero, nullity).is_err());

        let mut duplicate = producer.null_space_basis().to_vec();
        duplicate[1] = duplicate[0].clone();
        assert!(cross_check_basis(spec.id, &columns, &duplicate, nullity).is_err());

        let mut incomplete = producer.null_space_basis().to_vec();
        incomplete.pop();
        assert!(cross_check_basis(spec.id, &columns, &incomplete, nullity).is_err());
    }

    #[test]
    fn row_reduction_handles_i128_minimum_without_panicking() {
        let mut row = [i128::MIN, 0];
        reduce_integer_row(&mut row).unwrap();
        assert_eq!(row, [-1, 0]);
    }

    #[test]
    fn production_floor_budgets_receive_a_sealed_cross_check() {
        let first = verify_floor_pi_budgets(&physical_invariant_receipts()).unwrap();
        let second = verify_floor_pi_budgets(&physical_invariant_receipts()).unwrap();
        assert_eq!(first, second);
        assert_ne!(first.digest(), [0; 32]);
    }

    #[test]
    fn a_raised_budget_is_refused_by_both_algorithm_result_and_seal() {
        let mut receipts = physical_invariant_receipts();
        receipts[0].buckingham_pi_groups += 1;
        assert!(matches!(
            verify_floor_pi_budgets(&receipts),
            Err(AuthorityWatchdogError::BudgetDisagreement { .. })
        ));
    }

    #[test]
    fn phenomenon_membership_and_residual_slot_mutations_refuse() {
        let mut changed_phenomenon = physical_invariant_receipts();
        changed_phenomenon[0].phenomenon = "authored.unknown".to_owned();
        assert!(matches!(
            verify_floor_pi_budgets(&changed_phenomenon),
            Err(AuthorityWatchdogError::UnknownPhenomenon(_))
        ));

        let mut duplicate_slot = physical_invariant_receipts();
        duplicate_slot[2].residual_slot = duplicate_slot[1].residual_slot.clone();
        assert!(matches!(
            verify_floor_pi_budgets(&duplicate_slot),
            Err(AuthorityWatchdogError::DuplicateResidualSlot { .. })
        ));
    }

    #[test]
    fn omitted_duplicate_and_reordered_variables_refuse() {
        const OMITTED: [&str; 4] = ["G", "h", "c", "Delta_nu_Cs"];
        let omitted_specs = [
            PHENOMENA[0],
            PhenomenonSpec {
                id: "gravity_matter.scale_basis",
                ordered_variables: &OMITTED,
            },
        ];
        assert!(matches!(
            verify_with_specs(
                &physical_invariant_receipts(),
                &omitted_specs,
                &EXPECTED_PI_RECEIPTS
            ),
            Err(AuthorityWatchdogError::BudgetDisagreement { .. })
        ));

        const DUPLICATE: [&str; 5] = ["G", "m_e", "h", "c", "c"];
        let duplicate_specs = [
            PHENOMENA[0],
            PhenomenonSpec {
                id: "gravity_matter.scale_basis",
                ordered_variables: &DUPLICATE,
            },
        ];
        assert!(matches!(
            verify_with_specs(
                &physical_invariant_receipts(),
                &duplicate_specs,
                &EXPECTED_PI_RECEIPTS
            ),
            Err(AuthorityWatchdogError::Producer(
                DimensionAnalysisError::DuplicateColumnId(_)
            ))
        ));

        const REORDERED: [&str; 5] = ["m_e", "G", "h", "c", "Delta_nu_Cs"];
        let reordered_specs = [
            PHENOMENA[0],
            PhenomenonSpec {
                id: "gravity_matter.scale_basis",
                ordered_variables: &REORDERED,
            },
        ];
        assert!(matches!(
            verify_with_specs(
                &physical_invariant_receipts(),
                &reordered_specs,
                &EXPECTED_PI_RECEIPTS
            ),
            Err(AuthorityWatchdogError::ReceiptSealMismatch { .. })
        ));
    }

    #[test]
    fn a_dimension_exponent_mutation_changes_the_canonical_matrix_bytes() {
        let spec = PHENOMENA[1];
        let columns = columns_for(&spec).unwrap();
        let original = matrix_digest(&spec, &columns);
        let mut changed = columns.clone();
        changed[0] = SiDimensionColumn::new(
            changed[0].id(),
            crate::fundamentals::SiDimension::new(2, -1, -2, 0, 0, 0, 0),
        );
        assert_ne!(matrix_digest(&spec, &changed), original);
    }
}

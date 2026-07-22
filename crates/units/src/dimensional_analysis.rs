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

//! Exact rank, null-space, and target-dimension projection over the canonical
//! seven-coordinate SI representation.
//!
//! This module proves dimensional statements only. A target dimension lying in
//! the input span does not prove a physical law, choose a dimensionless
//! coefficient, or admit a value. The null space makes that remaining freedom
//! explicit so a caller cannot mistake unit closure for physical closure.

use crate::fundamentals::{SiDimension, SI_BASE_DIMENSION_IDS};
use std::{collections::BTreeSet, fmt};

/// One exact rational exponent in a dimensional relation.
///
/// The denominator is positive and the pair is always reduced. The SI matrix
/// has seven rows of `i8` exponents, so checked `i128` arithmetic is ample for
/// its minors while still failing closed on an unexpected expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExactExponent {
    numerator: i128,
    denominator: i128,
}

impl ExactExponent {
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    /// Construct and reduce one rational exponent.
    pub fn from_ratio(numerator: i128, denominator: i128) -> Result<Self, DimensionAnalysisError> {
        if denominator == 0 {
            return Err(DimensionAnalysisError::ZeroDenominator);
        }
        if numerator == 0 {
            return Ok(Self::ZERO);
        }
        let negative = numerator.is_negative() ^ denominator.is_negative();
        let divisor = gcd_u128(numerator.unsigned_abs(), denominator.unsigned_abs());
        let numerator_magnitude = numerator.unsigned_abs() / divisor;
        let denominator_magnitude = denominator.unsigned_abs() / divisor;
        Ok(Self {
            numerator: signed_magnitude(numerator_magnitude, negative)?,
            denominator: i128::try_from(denominator_magnitude)
                .map_err(|_| DimensionAnalysisError::ArithmeticOverflow)?,
        })
    }

    pub const fn numerator(self) -> i128 {
        self.numerator
    }

    pub const fn denominator(self) -> i128 {
        self.denominator
    }

    pub const fn is_zero(self) -> bool {
        self.numerator == 0
    }

    fn from_i8(value: i8) -> Self {
        Self {
            numerator: i128::from(value),
            denominator: 1,
        }
    }

    fn checked_neg(self) -> Result<Self, DimensionAnalysisError> {
        Self::from_ratio(
            self.numerator
                .checked_neg()
                .ok_or(DimensionAnalysisError::ArithmeticOverflow)?,
            self.denominator,
        )
    }

    fn checked_add(self, other: Self) -> Result<Self, DimensionAnalysisError> {
        let common = gcd_u128(self.denominator as u128, other.denominator as u128);
        let common =
            i128::try_from(common).map_err(|_| DimensionAnalysisError::ArithmeticOverflow)?;
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_scale)
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(right_scale)
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
        let numerator = left
            .checked_add(right)
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
        Self::from_ratio(numerator, denominator)
    }

    fn checked_sub(self, other: Self) -> Result<Self, DimensionAnalysisError> {
        self.checked_add(other.checked_neg()?)
    }

    fn checked_mul(self, other: Self) -> Result<Self, DimensionAnalysisError> {
        let left_cancel = gcd_u128(self.numerator.unsigned_abs(), other.denominator as u128);
        let right_cancel = gcd_u128(other.numerator.unsigned_abs(), self.denominator as u128);
        let left_cancel =
            i128::try_from(left_cancel).map_err(|_| DimensionAnalysisError::ArithmeticOverflow)?;
        let right_cancel =
            i128::try_from(right_cancel).map_err(|_| DimensionAnalysisError::ArithmeticOverflow)?;
        let numerator = (self.numerator / left_cancel)
            .checked_mul(other.numerator / right_cancel)
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
        let denominator = (self.denominator / right_cancel)
            .checked_mul(other.denominator / left_cancel)
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
        Self::from_ratio(numerator, denominator)
    }

    fn checked_div(self, other: Self) -> Result<Self, DimensionAnalysisError> {
        if other.is_zero() {
            return Err(DimensionAnalysisError::ZeroDenominator);
        }
        let mut numerator_left = self.numerator.unsigned_abs();
        let mut numerator_right = other.numerator.unsigned_abs();
        let mut denominator_left = self.denominator as u128;
        let mut denominator_right = other.denominator as u128;

        let numerator_cancel = gcd_u128(numerator_left, numerator_right);
        numerator_left /= numerator_cancel;
        numerator_right /= numerator_cancel;
        let denominator_cancel = gcd_u128(denominator_right, denominator_left);
        denominator_right /= denominator_cancel;
        denominator_left /= denominator_cancel;

        let numerator_magnitude = numerator_left
            .checked_mul(denominator_right)
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
        let denominator_magnitude = denominator_left
            .checked_mul(numerator_right)
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
        let negative = self.numerator.is_negative() ^ other.numerator.is_negative();
        Self::from_ratio(
            signed_magnitude(numerator_magnitude, negative)?,
            i128::try_from(denominator_magnitude)
                .map_err(|_| DimensionAnalysisError::ArithmeticOverflow)?,
        )
    }
}

impl fmt::Display for ExactExponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

/// One ordered column in an SI dimension matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiDimensionColumn {
    id: String,
    dimension: SiDimension,
}

impl SiDimensionColumn {
    pub fn new(id: &str, dimension: SiDimension) -> Self {
        Self {
            id: id.to_owned(),
            dimension,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn dimension(&self) -> SiDimension {
        self.dimension
    }
}

/// The canonical exponent vector that gives a target's dimension from the
/// ordered input columns. It carries no magnitude or physical-law claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionProjection {
    exponents: Vec<ExactExponent>,
}

impl DimensionProjection {
    pub fn exponents(&self) -> &[ExactExponent] {
        &self.exponents
    }
}

/// Exact analysis of one ordered SI dimension matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiDimensionAnalysis {
    columns: Vec<SiDimensionColumn>,
    rank: usize,
    pivot_columns: Vec<usize>,
    null_space_basis: Vec<Vec<i128>>,
}

impl SiDimensionAnalysis {
    /// Analyze the supplied columns in their declared order.
    pub fn analyze(columns: &[SiDimensionColumn]) -> Result<Self, DimensionAnalysisError> {
        validate_columns(columns)?;
        let rref = rref(
            &columns
                .iter()
                .map(SiDimensionColumn::dimension)
                .collect::<Vec<_>>(),
        )?;
        let null_space_basis = primitive_null_space_basis(&rref, columns.len())?;
        Ok(Self {
            columns: columns.to_vec(),
            rank: rref.pivot_columns.len(),
            pivot_columns: rref.pivot_columns,
            null_space_basis,
        })
    }

    pub fn columns(&self) -> &[SiDimensionColumn] {
        &self.columns
    }

    pub const fn rank(&self) -> usize {
        self.rank
    }

    pub fn pivot_columns(&self) -> &[usize] {
        &self.pivot_columns
    }

    pub fn null_space_basis(&self) -> &[Vec<i128>] {
        &self.null_space_basis
    }

    pub fn nullity(&self) -> usize {
        self.null_space_basis.len()
    }

    /// Test an explicit relation against the matrix. This verifies dimensional
    /// cancellation only and does not admit the relation as physical law.
    pub fn relation_is_dimensionless(
        &self,
        coefficients: &[i128],
    ) -> Result<bool, DimensionAnalysisError> {
        if coefficients.len() != self.columns.len() {
            return Err(DimensionAnalysisError::CoefficientCountMismatch {
                expected: self.columns.len(),
                found: coefficients.len(),
            });
        }
        for base_index in 0..SI_BASE_DIMENSION_IDS.len() {
            let mut total = ExactExponent::ZERO;
            for (column, coefficient) in self.columns.iter().zip(coefficients) {
                let exponent = ExactExponent::from_i8(column.dimension().exponents()[base_index]);
                let coefficient = ExactExponent::from_ratio(*coefficient, 1)?;
                total = total.checked_add(coefficient.checked_mul(exponent)?)?;
            }
            if !total.is_zero() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Find one canonical dimensional projection of a target onto the input
    /// columns. `None` means the target dimension is outside their span.
    ///
    /// A returned projection is not a physical derivation. Dimensionless
    /// functions and coefficients remain unconstrained by this operation.
    pub fn project_dimension(
        &self,
        target: SiDimension,
    ) -> Result<Option<DimensionProjection>, DimensionAnalysisError> {
        let mut augmented: Vec<_> = self
            .columns
            .iter()
            .map(SiDimensionColumn::dimension)
            .collect();
        augmented.push(target);
        let augmented_rref = rref(&augmented)?;
        if augmented_rref.pivot_columns.len() != self.rank {
            return Ok(None);
        }
        let target_index = self.columns.len();
        let basis = rational_null_space_basis(&augmented_rref, augmented.len())?;
        let relation = basis
            .into_iter()
            .find(|candidate| candidate[target_index] == ExactExponent::ONE)
            .ok_or(DimensionAnalysisError::ProjectionInvariantViolation)?;
        let exponents = relation[..target_index]
            .iter()
            .map(|coefficient| coefficient.checked_neg())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(DimensionProjection { exponents }))
    }
}

/// Failure to construct or query an exact dimensional census.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimensionAnalysisError {
    EmptyColumnId,
    DuplicateColumnId(String),
    ZeroDenominator,
    ArithmeticOverflow,
    CoefficientCountMismatch { expected: usize, found: usize },
    ProjectionInvariantViolation,
}

impl fmt::Display for DimensionAnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyColumnId => f.write_str("SI dimension column identity is empty"),
            Self::DuplicateColumnId(id) => {
                write!(f, "duplicate SI dimension column identity '{id}'")
            }
            Self::ZeroDenominator => f.write_str("exact dimension exponent has zero denominator"),
            Self::ArithmeticOverflow => {
                f.write_str("exact SI dimension elimination exceeded the checked i128 domain")
            }
            Self::CoefficientCountMismatch { expected, found } => write!(
                f,
                "dimension relation has {found} coefficient(s), expected {expected}"
            ),
            Self::ProjectionInvariantViolation => {
                f.write_str("dependent target lacks its canonical null-space relation")
            }
        }
    }
}

impl std::error::Error for DimensionAnalysisError {}

fn validate_columns(columns: &[SiDimensionColumn]) -> Result<(), DimensionAnalysisError> {
    let mut seen = BTreeSet::new();
    for column in columns {
        if column.id().is_empty() {
            return Err(DimensionAnalysisError::EmptyColumnId);
        }
        if !seen.insert(column.id()) {
            return Err(DimensionAnalysisError::DuplicateColumnId(
                column.id().to_owned(),
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
struct Rref {
    rows: Vec<Vec<ExactExponent>>,
    pivot_columns: Vec<usize>,
}

fn rref(dimensions: &[SiDimension]) -> Result<Rref, DimensionAnalysisError> {
    let column_count = dimensions.len();
    let mut rows = (0..SI_BASE_DIMENSION_IDS.len())
        .map(|base_index| {
            dimensions
                .iter()
                .map(|dimension| ExactExponent::from_i8(dimension.exponents()[base_index]))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut pivot_columns = Vec::new();
    let mut pivot_row = 0;
    for column in 0..column_count {
        let Some(found_row) = (pivot_row..rows.len()).find(|row| !rows[*row][column].is_zero())
        else {
            continue;
        };
        rows.swap(pivot_row, found_row);
        let pivot = rows[pivot_row][column];
        for entry in &mut rows[pivot_row] {
            *entry = entry.checked_div(pivot)?;
        }
        let normalized_pivot = rows[pivot_row].clone();
        for (row_index, row) in rows.iter_mut().enumerate() {
            if row_index == pivot_row || row[column].is_zero() {
                continue;
            }
            let factor = row[column];
            for (entry, normalized) in row.iter_mut().zip(&normalized_pivot) {
                *entry = entry.checked_sub(factor.checked_mul(*normalized)?)?;
            }
        }
        pivot_columns.push(column);
        pivot_row += 1;
        if pivot_row == rows.len() {
            break;
        }
    }
    Ok(Rref {
        rows,
        pivot_columns,
    })
}

fn rational_null_space_basis(
    rref: &Rref,
    column_count: usize,
) -> Result<Vec<Vec<ExactExponent>>, DimensionAnalysisError> {
    let pivots: BTreeSet<_> = rref.pivot_columns.iter().copied().collect();
    let mut basis = Vec::new();
    for free_column in (0..column_count).filter(|column| !pivots.contains(column)) {
        let mut vector = vec![ExactExponent::ZERO; column_count];
        vector[free_column] = ExactExponent::ONE;
        for (row_index, pivot_column) in rref.pivot_columns.iter().copied().enumerate() {
            vector[pivot_column] = rref.rows[row_index][free_column].checked_neg()?;
        }
        basis.push(vector);
    }
    Ok(basis)
}

fn primitive_null_space_basis(
    rref: &Rref,
    column_count: usize,
) -> Result<Vec<Vec<i128>>, DimensionAnalysisError> {
    let mut primitive = rational_null_space_basis(rref, column_count)?
        .into_iter()
        .map(|vector| primitive_integer_vector(&vector))
        .collect::<Result<Vec<_>, _>>()?;
    primitive.sort();
    Ok(primitive)
}

fn primitive_integer_vector(vector: &[ExactExponent]) -> Result<Vec<i128>, DimensionAnalysisError> {
    let mut common_denominator = 1_i128;
    for coefficient in vector {
        let divisor = gcd_u128(
            common_denominator as u128,
            coefficient.denominator() as u128,
        );
        let divisor =
            i128::try_from(divisor).map_err(|_| DimensionAnalysisError::ArithmeticOverflow)?;
        common_denominator = (common_denominator / divisor)
            .checked_mul(coefficient.denominator())
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
    }
    let mut integers = vector
        .iter()
        .map(|coefficient| {
            coefficient
                .numerator()
                .checked_mul(common_denominator / coefficient.denominator())
                .ok_or(DimensionAnalysisError::ArithmeticOverflow)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let divisor = integers.iter().fold(0_u128, |current, value| {
        gcd_u128(current, value.unsigned_abs())
    });
    let divisor =
        i128::try_from(divisor).map_err(|_| DimensionAnalysisError::ArithmeticOverflow)?;
    for coefficient in &mut integers {
        *coefficient /= divisor;
    }
    if integers
        .iter()
        .find(|coefficient| **coefficient != 0)
        .is_some_and(|coefficient| coefficient.is_negative())
    {
        for coefficient in &mut integers {
            *coefficient = coefficient
                .checked_neg()
                .ok_or(DimensionAnalysisError::ArithmeticOverflow)?;
        }
    }
    Ok(integers)
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn signed_magnitude(magnitude: u128, negative: bool) -> Result<i128, DimensionAnalysisError> {
    if negative && magnitude == i128::MIN.unsigned_abs() {
        return Ok(i128::MIN);
    }
    let magnitude =
        i128::try_from(magnitude).map_err(|_| DimensionAnalysisError::ArithmeticOverflow)?;
    if negative {
        magnitude
            .checked_neg()
            .ok_or(DimensionAnalysisError::ArithmeticOverflow)
    } else {
        Ok(magnitude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fundamentals::{PHYSICAL_INVARIANTS, REPRESENTATION_DEFINITIONS};

    fn floor_columns() -> Vec<SiDimensionColumn> {
        REPRESENTATION_DEFINITIONS
            .iter()
            .chain(PHYSICAL_INVARIANTS.iter())
            .map(|definition| SiDimensionColumn::new(definition.symbol, definition.dimension))
            .collect()
    }

    #[test]
    fn the_floor_basis_has_seven_dimensions_and_three_pi_groups() {
        let analysis = SiDimensionAnalysis::analyze(&floor_columns()).unwrap();
        assert_eq!(analysis.rank(), 7);
        assert_eq!(analysis.nullity(), 3);
        assert_eq!(
            analysis
                .columns()
                .iter()
                .map(SiDimensionColumn::id)
                .collect::<Vec<_>>(),
            vec![
                "Delta_nu_Cs",
                "c",
                "h",
                "e",
                "k_B",
                "N_A",
                "K_cd",
                "alpha",
                "G",
                "m_e",
            ]
        );
        assert!(analysis
            .null_space_basis()
            .iter()
            .all(|relation| analysis.relation_is_dimensionless(relation).unwrap()));
    }

    #[test]
    fn the_typed_floor_receipt_groups_lie_in_the_computed_null_space() {
        let analysis = SiDimensionAnalysis::analyze(&floor_columns()).unwrap();
        let alpha = [0, 0, 0, 0, 0, 0, 0, 1, 0, 0];
        let gravitational = [0, -1, -1, 0, 0, 0, 0, 0, 1, 2];
        let clock_mass = [-1, 2, -1, 0, 0, 0, 0, 0, 0, 1];
        assert!(analysis.relation_is_dimensionless(&alpha).unwrap());
        assert!(analysis.relation_is_dimensionless(&gravitational).unwrap());
        assert!(analysis.relation_is_dimensionless(&clock_mass).unwrap());
    }

    #[test]
    fn every_si_target_is_spanned_without_becoming_a_physical_value() {
        let analysis = SiDimensionAnalysis::analyze(&floor_columns()).unwrap();
        let unfamiliar = SiDimension::new(5, -3, 4, 1, -2, 6, -7);
        let projection = analysis
            .project_dimension(unfamiliar)
            .unwrap()
            .expect("the seven representation definitions span every SI dimension");
        assert_eq!(projection.exponents().len(), floor_columns().len());

        let dimensionless = analysis
            .project_dimension(SiDimension::DIMENSIONLESS)
            .unwrap()
            .expect("dimensionless is in every dimensional span");
        assert!(dimensionless
            .exponents()
            .iter()
            .all(|exponent| exponent.is_zero()));
    }

    #[test]
    fn rational_normalization_reduces_signed_extremes_before_sign_canonicalization() {
        assert_eq!(
            ExactExponent::from_ratio(0, i128::MIN).unwrap(),
            ExactExponent::ZERO
        );
        assert_eq!(
            ExactExponent::from_ratio(i128::MIN, -2).unwrap(),
            ExactExponent::from_ratio(1_i128 << 126, 1).unwrap()
        );
        assert_eq!(
            ExactExponent::from_ratio(i128::MIN, 1).unwrap().numerator(),
            i128::MIN
        );
        assert_eq!(
            ExactExponent::from_ratio(1, i128::MIN),
            Err(DimensionAnalysisError::ArithmeticOverflow)
        );
        let minimum = ExactExponent::from_ratio(i128::MIN, 1).unwrap();
        assert_eq!(minimum.checked_div(minimum).unwrap(), ExactExponent::ONE);
    }

    #[test]
    fn rank_deficiency_and_bad_catalogs_fail_visibly() {
        let length_only = [SiDimensionColumn::new(
            "length",
            SiDimension::new(1, 0, 0, 0, 0, 0, 0),
        )];
        let analysis = SiDimensionAnalysis::analyze(&length_only).unwrap();
        assert_eq!(analysis.rank(), 1);
        assert!(analysis
            .project_dimension(SiDimension::new(0, 1, 0, 0, 0, 0, 0))
            .unwrap()
            .is_none());

        let duplicate = [
            SiDimensionColumn::new("same", SiDimension::DIMENSIONLESS),
            SiDimensionColumn::new("same", SiDimension::DIMENSIONLESS),
        ];
        assert_eq!(
            SiDimensionAnalysis::analyze(&duplicate),
            Err(DimensionAnalysisError::DuplicateColumnId("same".into()))
        );
    }
}

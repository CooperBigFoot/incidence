//! scalar_semantics : NumericalSemanticsVersion × OrderedBinary64Operation → FiniteCanonicalBinary64 ⊎ NumericalSemanticsError   (pure, input-ordered)

use crate::non_negative_amount::{NonNegativeAmount, NonNegativeAmountError};
use serde::{Deserialize, Serialize};

/// A caller-selected version of the numerical semantics.
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericalSemanticsVersion {
    /// Literal authoritative-order binary64 semantics.
    V1,
    /// The second protocol identity, retaining V1 ordered binary64 operations.
    V2,
}

/// A scalar operation whose identity is carried by numerical diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarOperation {
    Normalize,
    Add,
    Subtract,
    Multiply,
    Divide,
    Compare,
    Minimum,
    Maximum,
    Clamp,
    Truth,
    Select,
    InterpolateLinear,
}

/// A named finite binary64 comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarComparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// The position of an invalid operand in a scalar operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperandPosition {
    Value,
    Lhs,
    Rhs,
    Lower,
    Upper,
    Condition,
    WhenTrue,
    WhenFalse,
    X,
    X0,
    X1,
    Y0,
    Y1,
}

impl NumericalSemanticsVersion {
    fn operand(
        self,
        operation: ScalarOperation,
        position: OperandPosition,
        value: f64,
    ) -> Result<f64, NumericalSemanticsError> {
        if !value.is_finite() {
            return Err(NumericalSemanticsError::NonFiniteOperand {
                operation,
                position,
                bits: value.to_bits(),
            });
        }
        Ok(canonicalize_zero(value))
    }

    fn result(
        self,
        operation: ScalarOperation,
        value: f64,
    ) -> Result<f64, NumericalSemanticsError> {
        if !value.is_finite() {
            return Err(NumericalSemanticsError::NonFiniteResult {
                operation,
                bits: value.to_bits(),
            });
        }
        Ok(canonicalize_zero(value))
    }

    /// Converts a finite scalar to canonical binary64 form.
    ///
    /// # Errors
    ///
    /// Returns [`NumericalSemanticsError::NonFiniteOperand`] when `value` is non-finite.
    pub fn normalize(self, value: f64) -> Result<f64, NumericalSemanticsError> {
        self.operand(ScalarOperation::Normalize, OperandPosition::Value, value)
    }

    /// Adds two finite scalars in written operand order.
    pub fn add(self, lhs: f64, rhs: f64) -> Result<f64, NumericalSemanticsError> {
        let lhs = self.operand(ScalarOperation::Add, OperandPosition::Lhs, lhs)?;
        let rhs = self.operand(ScalarOperation::Add, OperandPosition::Rhs, rhs)?;
        self.result(ScalarOperation::Add, lhs + rhs)
    }

    /// Subtracts two finite scalars in written operand order.
    pub fn subtract(self, lhs: f64, rhs: f64) -> Result<f64, NumericalSemanticsError> {
        let lhs = self.operand(ScalarOperation::Subtract, OperandPosition::Lhs, lhs)?;
        let rhs = self.operand(ScalarOperation::Subtract, OperandPosition::Rhs, rhs)?;
        self.result(ScalarOperation::Subtract, lhs - rhs)
    }

    /// Multiplies two finite scalars in written operand order.
    pub fn multiply(self, lhs: f64, rhs: f64) -> Result<f64, NumericalSemanticsError> {
        let lhs = self.operand(ScalarOperation::Multiply, OperandPosition::Lhs, lhs)?;
        let rhs = self.operand(ScalarOperation::Multiply, OperandPosition::Rhs, rhs)?;
        self.result(ScalarOperation::Multiply, lhs * rhs)
    }

    /// Divides two finite scalars in written operand order.
    pub fn divide(self, lhs: f64, rhs: f64) -> Result<f64, NumericalSemanticsError> {
        let lhs = self.operand(ScalarOperation::Divide, OperandPosition::Lhs, lhs)?;
        let rhs = self.operand(ScalarOperation::Divide, OperandPosition::Rhs, rhs)?;
        self.result(ScalarOperation::Divide, lhs / rhs)
    }

    /// Applies one named comparison to two finite scalars.
    pub fn compare(
        self,
        comparison: ScalarComparison,
        lhs: f64,
        rhs: f64,
    ) -> Result<bool, NumericalSemanticsError> {
        let lhs = self.operand(ScalarOperation::Compare, OperandPosition::Lhs, lhs)?;
        let rhs = self.operand(ScalarOperation::Compare, OperandPosition::Rhs, rhs)?;
        Ok(match comparison {
            ScalarComparison::Equal => lhs == rhs,
            ScalarComparison::NotEqual => lhs != rhs,
            ScalarComparison::LessThan => lhs < rhs,
            ScalarComparison::LessThanOrEqual => lhs <= rhs,
            ScalarComparison::GreaterThan => lhs > rhs,
            ScalarComparison::GreaterThanOrEqual => lhs >= rhs,
        })
    }

    /// Returns the lesser finite scalar, choosing the left operand on ties.
    pub fn minimum(self, lhs: f64, rhs: f64) -> Result<f64, NumericalSemanticsError> {
        let lhs = self.operand(ScalarOperation::Minimum, OperandPosition::Lhs, lhs)?;
        let rhs = self.operand(ScalarOperation::Minimum, OperandPosition::Rhs, rhs)?;
        Ok(if lhs <= rhs { lhs } else { rhs })
    }

    /// Returns the greater finite scalar, choosing the left operand on ties.
    pub fn maximum(self, lhs: f64, rhs: f64) -> Result<f64, NumericalSemanticsError> {
        let lhs = self.operand(ScalarOperation::Maximum, OperandPosition::Lhs, lhs)?;
        let rhs = self.operand(ScalarOperation::Maximum, OperandPosition::Rhs, rhs)?;
        Ok(if lhs >= rhs { lhs } else { rhs })
    }

    /// Restricts a finite scalar to inclusive finite bounds.
    pub fn clamp(self, value: f64, lower: f64, upper: f64) -> Result<f64, NumericalSemanticsError> {
        let value = self.operand(ScalarOperation::Clamp, OperandPosition::Value, value)?;
        let lower = self.operand(ScalarOperation::Clamp, OperandPosition::Lower, lower)?;
        let upper = self.operand(ScalarOperation::Clamp, OperandPosition::Upper, upper)?;
        if lower > upper {
            return Err(NumericalSemanticsError::InvalidClampBounds {
                lower_bits: lower.to_bits(),
                upper_bits: upper.to_bits(),
            });
        }
        let at_least_lower = self.maximum(value, lower)?;
        self.minimum(at_least_lower, upper)
    }

    /// Interprets canonical zero as false and every other finite scalar as true.
    pub fn truth(self, value: f64) -> Result<bool, NumericalSemanticsError> {
        let value = self.operand(ScalarOperation::Truth, OperandPosition::Value, value)?;
        Ok(value.to_bits() != 0)
    }

    /// Selects one of two finite candidates using exact scalar truth semantics.
    pub fn select(
        self,
        condition: f64,
        when_true: f64,
        when_false: f64,
    ) -> Result<f64, NumericalSemanticsError> {
        let condition = self.operand(
            ScalarOperation::Select,
            OperandPosition::Condition,
            condition,
        )?;
        let when_true = self.operand(
            ScalarOperation::Select,
            OperandPosition::WhenTrue,
            when_true,
        )?;
        let when_false = self.operand(
            ScalarOperation::Select,
            OperandPosition::WhenFalse,
            when_false,
        )?;
        Ok(if condition.to_bits() == 0 {
            when_false
        } else {
            when_true
        })
    }

    /// Interpolates linearly using six separately ordered binary64 operations.
    pub fn interpolate_linear(
        self,
        x: f64,
        x0: f64,
        x1: f64,
        y0: f64,
        y1: f64,
    ) -> Result<f64, NumericalSemanticsError> {
        let x = self.operand(ScalarOperation::InterpolateLinear, OperandPosition::X, x)?;
        let x0 = self.operand(ScalarOperation::InterpolateLinear, OperandPosition::X0, x0)?;
        let x1 = self.operand(ScalarOperation::InterpolateLinear, OperandPosition::X1, x1)?;
        let y0 = self.operand(ScalarOperation::InterpolateLinear, OperandPosition::Y0, y0)?;
        let y1 = self.operand(ScalarOperation::InterpolateLinear, OperandPosition::Y1, y1)?;
        if x0 >= x1 {
            return Err(NumericalSemanticsError::InvalidInterpolationAbscissae {
                x0_bits: x0.to_bits(),
                x1_bits: x1.to_bits(),
            });
        }
        let x_delta = self.subtract(x, x0)?;
        let width = self.subtract(x1, x0)?;
        let fraction = self.divide(x_delta, width)?;
        let y_delta = self.subtract(y1, y0)?;
        let scaled_delta = self.multiply(fraction, y_delta)?;
        self.add(y0, scaled_delta)
    }

    /// Accumulates amounts according to the selected numerical semantics.
    ///
    /// # Errors
    ///
    /// Returns [`AccumulationError::NonFiniteSum`] immediately when an addition produces a
    /// non-finite result. Returns [`AccumulationError::InvalidResult`] if the final finite result
    /// is unexpectedly rejected as a non-negative amount.
    pub fn accumulate<I>(self, amounts: I) -> Result<NonNegativeAmount, AccumulationError>
    where
        I: IntoIterator<Item = NonNegativeAmount>,
    {
        let mut accumulator = 0.0;
        for (index, amount) in amounts.into_iter().enumerate() {
            let partial_sum = accumulator;
            let addend = amount.value();
            accumulator =
                self.add(partial_sum, addend)
                    .map_err(|_| AccumulationError::NonFiniteSum {
                        version: self,
                        index,
                        partial_sum,
                        addend,
                    })?;
        }
        NonNegativeAmount::try_from(accumulator).map_err(|source| {
            AccumulationError::InvalidResult {
                version: self,
                source,
            }
        })
    }
}

fn canonicalize_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

/// An error applying finite ordered binary64 scalar semantics.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum NumericalSemanticsError {
    /// Fires when a scalar operand is NaN or infinite.
    #[error("non-finite operand at {position:?} for {operation:?}: bits {bits:#018x}")]
    NonFiniteOperand {
        operation: ScalarOperation,
        position: OperandPosition,
        bits: u64,
    },
    /// Fires when finite arithmetic produces NaN or infinity.
    #[error("non-finite result for {operation:?}: bits {bits:#018x}")]
    NonFiniteResult {
        operation: ScalarOperation,
        bits: u64,
    },
    /// Fires when the lower clamp bound is greater than the upper bound.
    #[error("invalid clamp bounds: lower bits {lower_bits:#018x}, upper bits {upper_bits:#018x}")]
    InvalidClampBounds { lower_bits: u64, upper_bits: u64 },
    /// Fires when interpolation abscissae are equal or decreasing.
    #[error("invalid interpolation abscissae: x0 bits {x0_bits:#018x}, x1 bits {x1_bits:#018x}")]
    InvalidInterpolationAbscissae { x0_bits: u64, x1_bits: u64 },
}

/// An error accumulating non-negative amounts.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AccumulationError {
    /// Fires when an addition first produces a non-finite sum.
    #[error(
        "accumulation for {version:?} became non-finite at index {index}: {partial_sum} + {addend}"
    )]
    NonFiniteSum {
        version: NumericalSemanticsVersion,
        index: usize,
        partial_sum: f64,
        addend: f64,
    },
    /// Fires when the checked final result is rejected as a non-negative amount.
    #[error("accumulation for {version:?} produced an invalid result: {source}")]
    InvalidResult {
        version: NumericalSemanticsVersion,
        #[source]
        source: NonNegativeAmountError,
    },
}

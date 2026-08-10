//! numerical_semantics : NumericalSemanticsVersion × NonNegativeAmount* → NonNegativeAmount ⊎ AccumulationError   (pure, input-ordered)

use crate::non_negative_amount::{NonNegativeAmount, NonNegativeAmountError};

/// A caller-selected version of the numerical semantics.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NumericalSemanticsVersion {
    /// Literal authoritative-order binary64 accumulation.
    V1,
}

impl NumericalSemanticsVersion {
    /// Accumulates amounts according to the selected numerical semantics.
    ///
    /// # Errors
    ///
    /// Returns [`AccumulationError::NonFiniteSum`] immediately when an
    /// addition produces a non-finite result. Returns
    /// [`AccumulationError::InvalidResult`] if the final finite result is
    /// unexpectedly rejected as a non-negative amount.
    pub fn accumulate<I>(self, amounts: I) -> Result<NonNegativeAmount, AccumulationError>
    where
        I: IntoIterator<Item = NonNegativeAmount>,
    {
        let mut accumulator = 0.0;
        for (index, amount) in amounts.into_iter().enumerate() {
            let partial_sum = accumulator;
            let addend = amount.value();
            accumulator = accumulator + addend;
            if !accumulator.is_finite() {
                return Err(AccumulationError::NonFiniteSum {
                    version: self,
                    index,
                    partial_sum,
                    addend,
                });
            }
        }

        if accumulator == 0.0 {
            return Ok(NonNegativeAmount::ZERO);
        }

        NonNegativeAmount::try_from(accumulator).map_err(|source| {
            AccumulationError::InvalidResult {
                version: self,
                source,
            }
        })
    }
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

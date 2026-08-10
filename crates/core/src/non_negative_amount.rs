//! non_negative_amount : IEEE754Binary64 → NonNegativeAmount ⊎ NonNegativeAmountError   (pure)

/// A finite, non-negative extensive amount.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct NonNegativeAmount(f64);

impl NonNegativeAmount {
    /// Canonical positive zero.
    pub const ZERO: Self = Self(0.0);

    /// Returns the finite, non-negative value.
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// An error constructing a [`NonNegativeAmount`] from a raw value.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum NonNegativeAmountError {
    /// Fires when the input is NaN or either infinity.
    #[error("non-negative amount must be finite, got {value}")]
    NonFinite { value: f64 },
    /// Fires when the input is finite and strictly less than zero.
    #[error("non-negative amount cannot be negative, got {value}")]
    Negative { value: f64 },
}

impl TryFrom<f64> for NonNegativeAmount {
    type Error = NonNegativeAmountError;

    /// Parses a finite, non-negative amount and canonicalizes signed zero.
    ///
    /// # Errors
    ///
    /// Returns [`NonNegativeAmountError::NonFinite`] for NaN or infinity and
    /// [`NonNegativeAmountError::Negative`] for a finite value below zero.
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(NonNegativeAmountError::NonFinite { value });
        }
        if value < 0.0 {
            return Err(NonNegativeAmountError::Negative { value });
        }
        if value == 0.0 {
            return Ok(Self::ZERO);
        }
        Ok(Self(value))
    }
}

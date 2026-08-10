//! signed_boundary_balance : IEEE754Binary64 → SignedBoundaryBalance ⊎ SignedBoundaryBalanceError   (pure)

/// A finite signed boundary-ledger balance.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SignedBoundaryBalance(f64);

impl SignedBoundaryBalance {
    /// Canonical positive zero.
    pub const ZERO: Self = Self(0.0);

    /// Returns the finite signed value.
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// An error constructing a [`SignedBoundaryBalance`] from a raw value.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SignedBoundaryBalanceError {
    /// Fires when the input is NaN or either infinity.
    #[error("signed boundary balance must be finite, got {value}")]
    NonFinite { value: f64 },
}

impl TryFrom<f64> for SignedBoundaryBalance {
    type Error = SignedBoundaryBalanceError;

    /// Parses a finite signed boundary balance and canonicalizes signed zero.
    ///
    /// # Errors
    ///
    /// Returns [`SignedBoundaryBalanceError::NonFinite`] when the input is NaN
    /// or either infinity.
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(SignedBoundaryBalanceError::NonFinite { value });
        }
        if value == 0.0 {
            return Ok(Self::ZERO);
        }
        Ok(Self(value))
    }
}

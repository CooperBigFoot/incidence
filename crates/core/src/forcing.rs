//! forcing_series : ForcingId × RunHorizon × FiniteScalar* ⇀ ForcingSeries   (pure, deterministic)

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::canonical_encoding::{
    CanonicalEncode, CanonicalEncodingError, CanonicalField, CanonicalPayloadWriter,
};
use crate::numerical_semantics::NumericalSemanticsVersion;
use crate::presence::ValueState;
use crate::rule_reference::ForcingId;
use crate::temporal::{RunHorizon, TimestepIndex};

/// An immutable finite scalar series covering one inclusive run horizon.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForcingSeries {
    id: ForcingId,
    horizon: RunHorizon,
    values: Box<[u64]>,
}

impl ForcingSeries {
    /// Constructs a forcing series after checking its shape and scalar values.
    ///
    /// # Errors
    ///
    /// Returns [`ForcingSeriesError::ValueCountMismatch`] when the number of values differs from
    /// the inclusive horizon length, or [`ForcingSeriesError::NonFiniteValue`] for a non-finite
    /// scalar.
    pub fn new(
        id: ForcingId,
        horizon: RunHorizon,
        values: Vec<f64>,
    ) -> Result<Self, ForcingSeriesError> {
        let first_timestep = horizon.first().value();
        let last_timestep = horizon.last().value();
        let expected = u128::from(last_timestep) - u128::from(first_timestep) + 1;
        if u128::try_from(values.len()).ok() != Some(expected) {
            return Err(ForcingSeriesError::ValueCountMismatch {
                first_timestep,
                last_timestep,
                expected,
                actual: values.len(),
            });
        }

        let mut bits = Vec::with_capacity(values.len());
        for (offset, value) in values.into_iter().enumerate() {
            let offset = match u64::try_from(offset) {
                Ok(offset) => offset,
                Err(_) => panic!("forcing value offset cannot be represented as a timestep"),
            };
            let timestep = match first_timestep.checked_add(offset) {
                Some(timestep) => timestep,
                None => panic!("validated forcing horizon offset exceeds its last timestep"),
            };
            let normalized = NumericalSemanticsVersion::V1
                .normalize(value)
                .map_err(|_| ForcingSeriesError::NonFiniteValue {
                    timestep,
                    bits: value.to_bits(),
                })?;
            bits.push(normalized.to_bits());
        }

        Ok(Self {
            id,
            horizon,
            values: bits.into_boxed_slice(),
        })
    }

    /// Returns the forcing identity.
    #[must_use]
    pub fn id(&self) -> &ForcingId {
        &self.id
    }

    /// Returns the inclusive run horizon.
    #[must_use]
    pub fn horizon(&self) -> RunHorizon {
        self.horizon
    }

    /// Returns the number of timestep values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Reports whether the series has no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Iterates over values in authored timestep order.
    pub fn values(&self) -> impl ExactSizeIterator<Item = f64> + '_ {
        self.values.iter().copied().map(f64::from_bits)
    }

    /// Returns the value state at one timestep.
    #[must_use]
    pub fn value_at(&self, timestep: TimestepIndex) -> ValueState<f64> {
        if !self.horizon.contains(timestep) {
            return ValueState::Absent;
        }
        let offset = match timestep.value().checked_sub(self.horizon.first().value()) {
            Some(offset) => offset,
            None => panic!("in-horizon forcing timestep precedes the horizon start"),
        };
        let index = match usize::try_from(offset) {
            Ok(index) => index,
            Err(_) => panic!("in-horizon forcing offset cannot be indexed"),
        };
        match self.values.get(index) {
            Some(bits) => ValueState::Present(f64::from_bits(*bits)),
            None => panic!("in-horizon forcing timestep has no constructed value"),
        }
    }
}

/// Reports why a forcing series could not be constructed.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum ForcingSeriesError {
    /// Fires when the supplied value count differs from the inclusive horizon length.
    #[error(
        "forcing horizon {first_timestep}..={last_timestep} requires {expected} values, received {actual}"
    )]
    ValueCountMismatch {
        first_timestep: u64,
        last_timestep: u64,
        expected: u128,
        actual: usize,
    },
    /// Fires when a supplied value is NaN or infinite.
    #[error("forcing value at timestep {timestep} is non-finite: bits {bits:#018x}")]
    NonFiniteValue { timestep: u64, bits: u64 },
}

#[derive(Serialize)]
struct ForcingSeriesWireRef<'a> {
    id: &'a ForcingId,
    horizon: RunHorizonWire,
    values: Vec<f64>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RunHorizonWire {
    first: u64,
    last: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ForcingSeriesWire {
    id: ForcingId,
    horizon: RunHorizonWire,
    values: Vec<f64>,
}

impl Serialize for ForcingSeries {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ForcingSeriesWireRef {
            id: &self.id,
            horizon: RunHorizonWire {
                first: self.horizon.first().value(),
                last: self.horizon.last().value(),
            },
            values: self.values().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ForcingSeries {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = ForcingSeriesWire::deserialize(deserializer)?;
        let horizon = RunHorizon::new(
            TimestepIndex::new(wire.horizon.first),
            TimestepIndex::new(wire.horizon.last),
        )
        .map_err(serde::de::Error::custom)?;
        Self::new(wire.id, horizon, wire.values).map_err(serde::de::Error::custom)
    }
}

impl CanonicalEncode for ForcingSeries {
    fn root_tag(&self) -> u16 {
        0x0018
    }

    fn encode_payload(
        &self,
        writer: &mut CanonicalPayloadWriter,
    ) -> Result<(), CanonicalEncodingError> {
        writer.write_string(CanonicalField::ForcingSeriesIdentity, self.id.as_str())?;
        writer.write_u64(self.horizon.first().value());
        writer.write_u64(self.horizon.last().value());
        writer.write_count(CanonicalField::ForcingSeriesValues, self.values.len())?;
        for value in self.values() {
            writer.write_scalar(CanonicalField::ForcingSeriesValue, value)?;
        }
        Ok(())
    }
}

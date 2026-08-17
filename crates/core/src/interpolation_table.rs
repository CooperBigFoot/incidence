//! interpolation_table : TableId × NumericalSemanticsVersion × InterpolationBoundaryPolicy × (FiniteAbscissa × FiniteOrdinate){2,} ⇀ InterpolationTable   (pure, deterministic)

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::canonical_encoding::{
    CanonicalEncode, CanonicalEncodingError, CanonicalField, CanonicalPayloadWriter,
};
use crate::numerical_semantics::NumericalSemanticsVersion;
use crate::rule_reference::TableId;

/// The closed behavior declared for inputs outside a table's endpoint interval.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationBoundaryPolicy {
    Reject,
    ClampToEndpoint,
    LinearExtrapolate,
}

/// An immutable ordered finite interpolation table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterpolationTable {
    id: TableId,
    numerical_semantics_version: NumericalSemanticsVersion,
    boundary_policy: InterpolationBoundaryPolicy,
    abscissae: Box<[u64]>,
    ordinates: Box<[u64]>,
}

impl InterpolationTable {
    /// Constructs an interpolation table after checking coordinate shape, finiteness, and order.
    ///
    /// # Errors
    ///
    /// Returns [`InterpolationTableError`] for the first failed invariant in contractual
    /// validation order.
    pub fn new(
        id: TableId,
        numerical_semantics_version: NumericalSemanticsVersion,
        boundary_policy: InterpolationBoundaryPolicy,
        abscissae: Vec<f64>,
        ordinates: Vec<f64>,
    ) -> Result<Self, InterpolationTableError> {
        if abscissae.len() != ordinates.len() {
            return Err(InterpolationTableError::CoordinateCountMismatch {
                abscissae: abscissae.len(),
                ordinates: ordinates.len(),
            });
        }
        if abscissae.len() < 2 {
            return Err(InterpolationTableError::TooFewPoints {
                points: abscissae.len(),
            });
        }

        let mut abscissa_bits = Vec::with_capacity(abscissae.len());
        for (index, value) in abscissae.into_iter().enumerate() {
            let normalized = numerical_semantics_version.normalize(value).map_err(|_| {
                InterpolationTableError::NonFiniteAbscissa {
                    index,
                    bits: value.to_bits(),
                }
            })?;
            abscissa_bits.push(normalized.to_bits());
        }
        let mut ordinate_bits = Vec::with_capacity(ordinates.len());
        for (index, value) in ordinates.into_iter().enumerate() {
            let normalized = numerical_semantics_version.normalize(value).map_err(|_| {
                InterpolationTableError::NonFiniteOrdinate {
                    index,
                    bits: value.to_bits(),
                }
            })?;
            ordinate_bits.push(normalized.to_bits());
        }
        for index in 0..abscissa_bits.len() - 1 {
            let left = f64::from_bits(abscissa_bits[index]);
            let right = f64::from_bits(abscissa_bits[index + 1]);
            if left >= right {
                return Err(InterpolationTableError::AbscissaeNotStrictlyIncreasing {
                    left_index: index,
                    right_index: index + 1,
                    left_bits: abscissa_bits[index],
                    right_bits: abscissa_bits[index + 1],
                });
            }
        }

        Ok(Self {
            id,
            numerical_semantics_version,
            boundary_policy,
            abscissae: abscissa_bits.into_boxed_slice(),
            ordinates: ordinate_bits.into_boxed_slice(),
        })
    }

    #[must_use]
    pub fn id(&self) -> &TableId {
        &self.id
    }

    #[must_use]
    pub fn numerical_semantics_version(&self) -> NumericalSemanticsVersion {
        self.numerical_semantics_version
    }

    #[must_use]
    pub fn boundary_policy(&self) -> InterpolationBoundaryPolicy {
        self.boundary_policy
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.abscissae.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.abscissae.is_empty()
    }

    pub fn points(&self) -> impl ExactSizeIterator<Item = (f64, f64)> + '_ {
        self.abscissae
            .iter()
            .copied()
            .zip(self.ordinates.iter().copied())
            .map(|(x, y)| (f64::from_bits(x), f64::from_bits(y)))
    }
}

/// Reports why an interpolation table could not be constructed.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum InterpolationTableError {
    /// Fires when the abscissa and ordinate counts differ.
    #[error("interpolation table has {abscissae} abscissae but {ordinates} ordinates")]
    CoordinateCountMismatch { abscissae: usize, ordinates: usize },
    /// Fires when fewer than two coordinate pairs are supplied.
    #[error("interpolation table requires at least two points, received {points}")]
    TooFewPoints { points: usize },
    /// Fires when an abscissa is NaN or infinite.
    #[error("interpolation table abscissa at index {index} is non-finite: bits {bits:#018x}")]
    NonFiniteAbscissa { index: usize, bits: u64 },
    /// Fires when an ordinate is NaN or infinite.
    #[error("interpolation table ordinate at index {index} is non-finite: bits {bits:#018x}")]
    NonFiniteOrdinate { index: usize, bits: u64 },
    /// Fires when adjacent abscissae are equal or decreasing.
    #[error(
        "interpolation table abscissae are not strictly increasing at indices {left_index} and {right_index}: bits {left_bits:#018x}, {right_bits:#018x}"
    )]
    AbscissaeNotStrictlyIncreasing {
        left_index: usize,
        right_index: usize,
        left_bits: u64,
        right_bits: u64,
    },
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum WireVersion {
    V1,
}

#[derive(Serialize)]
struct InterpolationTableWireRef<'a> {
    id: &'a TableId,
    numerical_semantics_version: WireVersion,
    boundary_policy: InterpolationBoundaryPolicy,
    abscissae: Vec<f64>,
    ordinates: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InterpolationTableWire {
    id: TableId,
    numerical_semantics_version: WireVersion,
    boundary_policy: InterpolationBoundaryPolicy,
    abscissae: Vec<f64>,
    ordinates: Vec<f64>,
}

impl Serialize for InterpolationTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (abscissae, ordinates): (Vec<_>, Vec<_>) = self.points().unzip();
        InterpolationTableWireRef {
            id: &self.id,
            numerical_semantics_version: WireVersion::V1,
            boundary_policy: self.boundary_policy,
            abscissae,
            ordinates,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InterpolationTable {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = InterpolationTableWire::deserialize(deserializer)?;
        let version = match wire.numerical_semantics_version {
            WireVersion::V1 => NumericalSemanticsVersion::V1,
        };
        Self::new(
            wire.id,
            version,
            wire.boundary_policy,
            wire.abscissae,
            wire.ordinates,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl CanonicalEncode for InterpolationTable {
    fn root_tag(&self) -> u16 {
        0x001b
    }

    fn encode_payload(
        &self,
        writer: &mut CanonicalPayloadWriter,
    ) -> Result<(), CanonicalEncodingError> {
        writer.write_string(CanonicalField::InterpolationTableIdentity, self.id.as_str())?;
        match self.numerical_semantics_version {
            NumericalSemanticsVersion::V1 => writer.write_u16(0x0001),
            NumericalSemanticsVersion::V2 => writer.write_u16(0x0002),
        }
        writer.write_u8(match self.boundary_policy {
            InterpolationBoundaryPolicy::Reject => 0x00,
            InterpolationBoundaryPolicy::ClampToEndpoint => 0x01,
            InterpolationBoundaryPolicy::LinearExtrapolate => 0x02,
        });
        writer.write_count(CanonicalField::InterpolationTablePoints, self.len())?;
        for (abscissa, ordinate) in self.points() {
            writer.write_scalar(CanonicalField::InterpolationTableAbscissa, abscissa)?;
            writer.write_scalar(CanonicalField::InterpolationTableOrdinate, ordinate)?;
        }
        Ok(())
    }
}

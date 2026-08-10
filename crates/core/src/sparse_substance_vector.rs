//! sparse_substance_vector : SubstanceRegistry × [(SubstanceId, NonNegativeAmount)] ⇀ SparseSubstanceVector   (canonical sparse storage, registry-ordered reduction).

use std::collections::{BTreeMap, BTreeSet};

use crate::identity::SubstanceId;
use crate::non_negative_amount::NonNegativeAmount;
use crate::numerical_semantics::{AccumulationError, NumericalSemanticsVersion};
use crate::presence::ValueState;
use crate::substance_registry::SubstanceRegistry;

/// A registry-bound sparse vector of non-negative substance amounts.
#[derive(Clone, Debug, PartialEq)]
pub struct SparseSubstanceVector {
    registry: SubstanceRegistry,
    entries: BTreeMap<SubstanceId, NonNegativeAmount>,
}

impl SparseSubstanceVector {
    /// Constructs a canonical sparse vector bound to a snapshot of `registry`.
    ///
    /// # Errors
    ///
    /// Returns [`SparseSubstanceVectorError::UnregisteredSubstance`] for the first identity outside
    /// the registry, or [`SparseSubstanceVectorError::DuplicateSubstance`] for the first repeated
    /// identity.
    pub fn new(
        registry: &SubstanceRegistry,
        entries: impl IntoIterator<Item = (SubstanceId, NonNegativeAmount)>,
    ) -> Result<Self, SparseSubstanceVectorError> {
        let mut seen = BTreeSet::new();
        let mut sparse_entries = BTreeMap::new();

        for (substance, amount) in entries {
            if !registry.contains(&substance) {
                return Err(SparseSubstanceVectorError::UnregisteredSubstance { substance });
            }
            if !seen.insert(substance.clone()) {
                return Err(SparseSubstanceVectorError::DuplicateSubstance { substance });
            }
            if amount.value().to_bits() != 0x0000000000000000 {
                sparse_entries.insert(substance, amount);
            }
        }

        Ok(Self {
            registry: registry.clone(),
            entries: sparse_entries,
        })
    }

    /// Returns the registry snapshot bound at construction.
    pub fn registry(&self) -> &SubstanceRegistry {
        &self.registry
    }

    /// Iterates over stored non-zero amounts in canonical registry order.
    pub fn iter(&self) -> impl Iterator<Item = (&SubstanceId, NonNegativeAmount)> {
        self.registry.iter().filter_map(|substance| {
            self.entries
                .get(substance)
                .map(|amount| (substance, *amount))
        })
    }

    /// Returns the amount state for `substance` under the bound registry.
    pub fn amount(&self, substance: &SubstanceId) -> ValueState<NonNegativeAmount> {
        if !self.registry.contains(substance) {
            return ValueState::NotModelled;
        }

        ValueState::Present(
            self.entries
                .get(substance)
                .copied()
                .unwrap_or(NonNegativeAmount::ZERO),
        )
    }

    /// Accumulates stored amounts in canonical registry order under `version`.
    ///
    /// # Errors
    ///
    /// Propagates [`AccumulationError`] when the selected numerical semantics cannot represent the
    /// sum.
    pub fn accumulate(
        &self,
        version: NumericalSemanticsVersion,
    ) -> Result<NonNegativeAmount, AccumulationError> {
        version.accumulate(self.iter().map(|(_, amount)| amount))
    }
}

/// Reports why a sparse substance vector could not be constructed.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SparseSubstanceVectorError {
    /// Fires when an input identity is outside the supplied registry.
    #[error("substance identity `{substance}` is not registered")]
    UnregisteredSubstance { substance: SubstanceId },
    /// Fires when an input identity has already appeared in the constructor input.
    #[error("duplicate substance identity `{substance}` in sparse vector")]
    DuplicateSubstance { substance: SubstanceId },
}

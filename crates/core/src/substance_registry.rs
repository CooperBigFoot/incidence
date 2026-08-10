//! substance_registry : [SubstanceId] ⇀ SubstanceRegistry   (unique membership, canonical order).

use std::collections::BTreeSet;

use thiserror::Error;

use crate::identity::SubstanceId;

/// A unique collection of substance identities in canonical order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubstanceRegistry {
    ids: BTreeSet<SubstanceId>,
}

impl SubstanceRegistry {
    /// Constructs a registry, rejecting the first repeated identity.
    ///
    /// # Errors
    ///
    /// Returns [`SubstanceRegistryError::DuplicateSubstance`] when an identity appears more than
    /// once.
    pub fn new(ids: impl IntoIterator<Item = SubstanceId>) -> Result<Self, SubstanceRegistryError> {
        let mut registry = BTreeSet::new();
        for substance in ids {
            if !registry.insert(substance.clone()) {
                return Err(SubstanceRegistryError::DuplicateSubstance { substance });
            }
        }
        Ok(Self { ids: registry })
    }

    /// Iterates over substance identities in ascending canonical order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &SubstanceId> + DoubleEndedIterator {
        self.ids.iter()
    }

    /// Returns whether the registry contains `substance`.
    pub fn contains(&self, substance: &SubstanceId) -> bool {
        self.ids.contains(substance)
    }

    /// Returns the number of substance identities in the registry.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Returns whether the registry contains no substance identities.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

/// Reports why a substance registry could not be constructed.
#[derive(Debug, Eq, Error, PartialEq)]
pub enum SubstanceRegistryError {
    /// Returned when an input repeats a substance identity already seen by the constructor.
    #[error("duplicate substance identity `{substance}`")]
    DuplicateSubstance { substance: SubstanceId },
}

#[cfg(test)]
mod tests {
    use crate::identity::SubstanceId;
    use crate::substance_registry::{SubstanceRegistry, SubstanceRegistryError};

    fn parse_ids(values: &[&str]) -> Vec<SubstanceId> {
        values
            .iter()
            .map(|value| SubstanceId::parse(value).expect("fixture identity must be valid"))
            .collect()
    }

    #[test]
    fn canonical_order_is_independent_of_insertion_order() {
        let first = SubstanceRegistry::new(parse_ids(&["water", "nitrate", "salt"]))
            .expect("fixture registry must be valid");
        let first_order: Vec<_> = first.iter().map(SubstanceId::as_str).collect();
        assert_eq!(first_order, ["nitrate", "salt", "water"]);

        let second = SubstanceRegistry::new(parse_ids(&["salt", "water", "nitrate"]))
            .expect("fixture registry must be valid");
        let second_order: Vec<_> = second.iter().map(SubstanceId::as_str).collect();

        assert_eq!(second, first);
        assert_eq!(second_order, ["nitrate", "salt", "water"]);
    }

    #[test]
    fn rejects_first_repeated_identity() {
        let result = SubstanceRegistry::new(parse_ids(&["water", "salt", "water"]));
        let expected_duplicate =
            SubstanceId::parse("water").expect("fixture identity must be valid");

        assert_eq!(
            result,
            Err(SubstanceRegistryError::DuplicateSubstance {
                substance: expected_duplicate,
            })
        );
        assert_eq!(
            result
                .expect_err("duplicate fixture must be rejected")
                .to_string(),
            "duplicate substance identity `water`"
        );
    }

    #[test]
    fn exposes_size_and_membership_queries() {
        let registry = SubstanceRegistry::new(parse_ids(&["water", "nitrate", "salt"]))
            .expect("fixture registry must be valid");
        let water = SubstanceId::parse("water").expect("fixture identity must be valid");
        let phosphorus = SubstanceId::parse("phosphorus").expect("fixture identity must be valid");

        assert_eq!(registry.len(), 3);
        assert!(!registry.is_empty());
        assert!(registry.contains(&water));
        assert!(!registry.contains(&phosphorus));
    }

    #[test]
    fn permits_empty_registry() {
        let registry = SubstanceRegistry::new(Vec::<SubstanceId>::new())
            .expect("empty registry must be valid");

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        assert_eq!(registry.iter().next(), None);
    }
}

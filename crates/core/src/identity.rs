//! canonical_identity_grammar : str ⇀ CanonicalIdentityText   (pure)
//! compartment_identity : str ⇀ CompartmentId; substance_identity : str ⇀ SubstanceId   (pure)

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use thiserror::Error;

/// Identifies the domain whose identity failed validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityKind {
    /// A compartment identity.
    Compartment,
    /// A substance identity.
    Substance,
}

impl Display for IdentityKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compartment => formatter.write_str("compartment"),
            Self::Substance => formatter.write_str("substance"),
        }
    }
}

/// Reports why an identity could not be constructed.
#[derive(Debug, Eq, Error, PartialEq)]
pub enum IdentityError {
    /// Returned when an identity is empty.
    #[error("{kind} identity must not be empty")]
    Empty { kind: IdentityKind },

    /// Returned when the first byte is not an ASCII lowercase letter.
    #[error("{kind} identity `{value}` must start with an ASCII lowercase letter")]
    InvalidStart { kind: IdentityKind, value: String },

    /// Returned when a character after the first falls outside the identity grammar.
    #[error(
        "{kind} identity `{value}` contains invalid character `{character}` at byte {byte_index}"
    )]
    InvalidCharacter {
        kind: IdentityKind,
        value: String,
        byte_index: usize,
        character: char,
    },
}

pub(crate) struct CanonicalIdentityText(String);

impl CanonicalIdentityText {
    pub(crate) fn into_string(self) -> String {
        self.0
    }
}

pub(crate) enum IdentityGrammarError {
    Empty,
    InvalidStart {
        value: String,
    },
    InvalidCharacter {
        value: String,
        byte_index: usize,
        character: char,
    },
}

pub(crate) fn parse_canonical_identity(
    value: &str,
) -> Result<CanonicalIdentityText, IdentityGrammarError> {
    let mut characters = value.char_indices();
    let Some((_, first)) = characters.next() else {
        return Err(IdentityGrammarError::Empty);
    };

    if !first.is_ascii_lowercase() {
        return Err(IdentityGrammarError::InvalidStart {
            value: value.to_owned(),
        });
    }

    for (byte_index, character) in characters {
        if !(character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '-' | '_'))
        {
            return Err(IdentityGrammarError::InvalidCharacter {
                value: value.to_owned(),
                byte_index,
                character,
            });
        }
    }

    Ok(CanonicalIdentityText(value.to_owned()))
}

fn parse_identity(value: &str, kind: IdentityKind) -> Result<String, IdentityError> {
    parse_canonical_identity(value)
        .map(CanonicalIdentityText::into_string)
        .map_err(|error| match error {
            IdentityGrammarError::Empty => IdentityError::Empty { kind },
            IdentityGrammarError::InvalidStart { value } => {
                IdentityError::InvalidStart { kind, value }
            }
            IdentityGrammarError::InvalidCharacter {
                value,
                byte_index,
                character,
            } => IdentityError::InvalidCharacter {
                kind,
                value,
                byte_index,
                character,
            },
        })
}

/// A validated compartment identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompartmentId(String);

impl CompartmentId {
    /// Parses a compartment identity using the canonical identity grammar.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] when `value` is empty or contains a byte outside the grammar.
    pub fn parse(value: &str) -> Result<Self, IdentityError> {
        parse_identity(value, IdentityKind::Compartment).map(Self)
    }

    /// Returns the identity text exactly as supplied at construction.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for CompartmentId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for CompartmentId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CompartmentId {
    type Err = IdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for CompartmentId {
    type Error = IdentityError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

/// A validated substance identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SubstanceId(String);

impl SubstanceId {
    /// Parses a substance identity using the canonical identity grammar.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] when `value` is empty or contains a byte outside the grammar.
    pub fn parse(value: &str) -> Result<Self, IdentityError> {
        parse_identity(value, IdentityKind::Substance).map(Self)
    }

    /// Returns the identity text exactly as supplied at construction.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SubstanceId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for SubstanceId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for SubstanceId {
    type Err = IdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for SubstanceId {
    type Error = IdentityError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::{CompartmentId, IdentityError, IdentityKind, SubstanceId};

    #[test]
    fn accepts_and_preserves_valid_compartment_identity() {
        let identity =
            CompartmentId::parse("reservoir_01").expect("fixture identity must be valid");

        assert_eq!(identity.as_str(), "reservoir_01");
        assert_eq!(identity.to_string(), "reservoir_01");
    }

    #[test]
    fn accepts_and_preserves_valid_substance_identity() {
        let identity = SubstanceId::parse("salt-2").expect("fixture identity must be valid");

        assert_eq!(identity.as_str(), "salt-2");
        assert_eq!(identity.to_string(), "salt-2");
    }

    #[test]
    fn rejects_empty_compartment_identity() {
        let result = CompartmentId::parse("");

        assert_eq!(
            result,
            Err(IdentityError::Empty {
                kind: IdentityKind::Compartment,
            })
        );
        assert_eq!(
            result
                .expect_err("empty fixture must be rejected")
                .to_string(),
            "compartment identity must not be empty"
        );
    }

    #[test]
    fn rejects_compartment_identity_with_invalid_start() {
        assert_eq!(
            CompartmentId::parse("1reservoir"),
            Err(IdentityError::InvalidStart {
                kind: IdentityKind::Compartment,
                value: "1reservoir".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_substance_identity_with_invalid_start() {
        let result = SubstanceId::parse("Salt");

        assert_eq!(
            result,
            Err(IdentityError::InvalidStart {
                kind: IdentityKind::Substance,
                value: "Salt".to_owned(),
            })
        );
        assert_eq!(
            result
                .expect_err("uppercase fixture must be rejected")
                .to_string(),
            "substance identity `Salt` must start with an ASCII lowercase letter"
        );
    }

    #[test]
    fn rejects_invalid_ascii_character() {
        let result = SubstanceId::parse("sea salt");

        assert_eq!(
            result,
            Err(IdentityError::InvalidCharacter {
                kind: IdentityKind::Substance,
                value: "sea salt".to_owned(),
                byte_index: 3,
                character: ' ',
            })
        );
        assert_eq!(
            result
                .expect_err("space fixture must be rejected")
                .to_string(),
            "substance identity `sea salt` contains invalid character ` ` at byte 3"
        );
    }

    #[test]
    fn reports_non_ascii_character_at_utf8_byte_offset() {
        assert_eq!(
            SubstanceId::parse("naïve"),
            Err(IdentityError::InvalidCharacter {
                kind: IdentityKind::Substance,
                value: "naïve".to_owned(),
                byte_index: 2,
                character: 'ï',
            })
        );
    }

    #[test]
    fn permits_trailing_and_repeated_separators() {
        assert!(CompartmentId::parse("reservoir-").is_ok());
        assert!(SubstanceId::parse("salt__2").is_ok());
    }

    #[test]
    fn orders_identities_by_exact_stored_bytes() {
        let compartment_a = CompartmentId::parse("a").expect("fixture identity must be valid");
        let compartment_b = CompartmentId::parse("b").expect("fixture identity must be valid");
        let substance_a = SubstanceId::parse("a").expect("fixture identity must be valid");
        let substance_b = SubstanceId::parse("b").expect("fixture identity must be valid");

        assert!(compartment_a < compartment_b);
        assert!(substance_a < substance_b);
    }
}

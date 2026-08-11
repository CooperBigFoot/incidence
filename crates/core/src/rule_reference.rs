//! rule_reference : ValidatedIdentity × ExpressionValueKind → TypedOpaqueRuleReference   (pure)

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::identity::{CanonicalIdentityText, IdentityGrammarError, parse_canonical_identity};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionValueKind {
    Scalar,
    Truth,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RuleReferenceKind {
    Input,
    Parameter,
    Forcing,
    Projection,
    Table,
    TransferBranch,
}

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum RuleReferenceIdentityError {
    /// Fires when an identity has no text.
    #[error("{kind:?} identity is empty")]
    Empty { kind: RuleReferenceKind },
    /// Fires when an identity does not begin with an ASCII lowercase letter.
    #[error("{kind:?} identity {value:?} has an invalid first character")]
    InvalidStart {
        kind: RuleReferenceKind,
        value: String,
    },
    /// Fires when an identity contains a character outside its canonical grammar.
    #[error("{kind:?} identity {value:?} has invalid character {character:?} at byte {byte_index}")]
    InvalidCharacter {
        kind: RuleReferenceKind,
        value: String,
        byte_index: usize,
        character: char,
    },
}

fn parse_identity(
    kind: RuleReferenceKind,
    value: &str,
) -> Result<String, RuleReferenceIdentityError> {
    parse_canonical_identity(value)
        .map(CanonicalIdentityText::into_string)
        .map_err(|error| match error {
            IdentityGrammarError::Empty => RuleReferenceIdentityError::Empty { kind },
            IdentityGrammarError::InvalidStart { value } => {
                RuleReferenceIdentityError::InvalidStart { kind, value }
            }
            IdentityGrammarError::InvalidCharacter {
                value,
                byte_index,
                character,
            } => RuleReferenceIdentityError::InvalidCharacter {
                kind,
                value,
                byte_index,
                character,
            },
        })
}

macro_rules! identity {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);
        impl $name {
            /// Parses text using the canonical rule-reference identity grammar.
            ///
            /// # Errors
            ///
            /// Returns [`RuleReferenceIdentityError`] when the text violates the grammar.
            pub fn parse(value: &str) -> Result<Self, RuleReferenceIdentityError> {
                parse_identity(RuleReferenceKind::$kind, value).map(Self)
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
        impl FromStr for $name {
            type Err = RuleReferenceIdentityError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
        impl TryFrom<&str> for $name {
            type Error = RuleReferenceIdentityError;
            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                Self::parse(&value).map_err(serde::de::Error::custom)
            }
        }
    };
}

identity!(InputId, Input);
identity!(ParameterId, Parameter);
identity!(ForcingId, Forcing);
identity!(ProjectionId, Projection);
identity!(TableId, Table);
identity!(TransferBranchId, TransferBranch);

macro_rules! kind_reference {
    ($name:ident, $id:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            id: $id,
            value_kind: ExpressionValueKind,
        }
        impl $name {
            pub fn new(id: $id, value_kind: ExpressionValueKind) -> Self {
                Self { id, value_kind }
            }
            pub fn id(&self) -> &$id {
                &self.id
            }
            pub fn value_kind(&self) -> ExpressionValueKind {
                self.value_kind
            }
        }
    };
}
kind_reference!(InputRef, InputId);
kind_reference!(ParameterRef, ParameterId);
kind_reference!(ProjectionRef, ProjectionId);

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ForcingRef {
    id: ForcingId,
}
impl ForcingRef {
    pub fn new(id: ForcingId) -> Self {
        Self { id }
    }
    pub fn id(&self) -> &ForcingId {
        &self.id
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InterpolatedTableRef {
    id: TableId,
}
impl InterpolatedTableRef {
    pub fn new(id: TableId) -> Self {
        Self { id }
    }
    pub fn id(&self) -> &TableId {
        &self.id
    }
}

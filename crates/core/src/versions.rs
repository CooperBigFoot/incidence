//! version_values : ProtocolComponent → ClosedVersionIdentity   (pure)

use serde::{Deserialize, Serialize};

/// The selected rule intermediate-representation version.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleIrVersion {
    /// The first rule intermediate-representation contract.
    V1,
}

/// The selected interpreter version.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterpreterVersion {
    /// The first interpreter contract.
    V1,
}

/// The selected canonical-encoding version.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalEncodingVersion {
    /// The first canonical-encoding contract.
    V1,
}

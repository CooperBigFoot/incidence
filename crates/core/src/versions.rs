//! version_values : ProtocolComponent → ClosedVersionIdentity   (pure)

/// The selected rule intermediate-representation version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RuleIrVersion {
    /// The first rule intermediate-representation contract.
    V1,
}

/// The selected interpreter version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum InterpreterVersion {
    /// The first interpreter contract.
    V1,
}

/// The selected canonical-encoding version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CanonicalEncodingVersion {
    /// The first canonical-encoding contract.
    V1,
}

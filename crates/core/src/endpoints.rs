//! finite_endpoint : CompartmentId → FiniteCompartment; boundary_endpoint : CompartmentId → BoundaryAccount.

use crate::identity::CompartmentId;

/// An identity-bearing finite compartment endpoint.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FiniteCompartment(CompartmentId);

impl FiniteCompartment {
    /// Constructs a finite compartment endpoint.
    pub fn new(id: CompartmentId) -> Self {
        Self(id)
    }

    /// Returns the endpoint's compartment identity.
    pub fn id(&self) -> &CompartmentId {
        &self.0
    }

    /// Consumes the endpoint and returns its compartment identity.
    pub fn into_id(self) -> CompartmentId {
        self.0
    }
}

/// An identity-bearing boundary account endpoint.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BoundaryAccount(CompartmentId);

impl BoundaryAccount {
    /// Constructs a boundary account endpoint.
    pub fn new(id: CompartmentId) -> Self {
        Self(id)
    }

    /// Returns the endpoint's compartment identity.
    pub fn id(&self) -> &CompartmentId {
        &self.0
    }

    /// Consumes the endpoint and returns its compartment identity.
    pub fn into_id(self) -> CompartmentId {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use crate::endpoints::{BoundaryAccount, FiniteCompartment};
    use crate::identity::CompartmentId;

    #[test]
    fn endpoint_types_preserve_identity_and_are_structurally_distinct() {
        let id = CompartmentId::parse("reservoir").expect("fixture identity must be valid");
        let finite = FiniteCompartment::new(id.clone());
        let boundary = BoundaryAccount::new(id);

        assert_eq!(finite.id().as_str(), "reservoir");
        assert_eq!(boundary.id().as_str(), "reservoir");
        assert_eq!(
            finite.into_id(),
            CompartmentId::parse("reservoir").expect("fixture identity must be valid")
        );
        assert_eq!(
            boundary.into_id(),
            CompartmentId::parse("reservoir").expect("fixture identity must be valid")
        );
        assert_ne!(
            TypeId::of::<FiniteCompartment>(),
            TypeId::of::<BoundaryAccount>()
        );
    }
}

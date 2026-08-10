//! `incidence_core : () → ()` (pure conserved-flow domain boundary; no operations yet)

pub mod endpoints;
pub mod identity;
pub mod non_negative_amount;
pub mod numerical_semantics;
pub mod presence;
pub mod signed_boundary_balance;
pub mod substance_registry;
pub mod temporal;
pub mod topology;

pub mod workspace {
    //! `workspace_boundary : InitializedIncidenceWorkspace → CoreDomainBoundary` (pure)
}

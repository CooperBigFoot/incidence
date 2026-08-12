//! `incidence_core : () → ()` (pure conserved-flow domain boundary; no operations yet)

pub mod canonical_encoding;
pub mod endpoints;
pub mod identity;
pub mod initial_stocks;
pub mod non_negative_amount;
pub mod numerical_semantics;
pub mod partition_expression;
pub mod presence;
pub mod projection;
pub mod rule_expression;
pub mod rule_reference;
pub mod signed_boundary_balance;
pub mod sparse_substance_vector;
pub mod substance_registry;
pub mod temporal;
pub mod topology;
pub mod versions;

pub mod workspace {
    //! `workspace_boundary : InitializedIncidenceWorkspace → CoreDomainBoundary` (pure)
}

//! execution_bindings : RuleCoordinates × RuleReferences ⇀ DeclaredExecutionSources   (pure, deterministic)
//!
//! Bindings keep endpoint routing and external rule inputs in the immutable model value rather
//! than in executor code.

use std::collections::BTreeSet;

use crate::identity::{CompartmentId, SubstanceId};
use crate::rule_reference::{
    ExpressionValueKind, ForcingRef, InputId, InputRef, InterpolatedTableRef, ProjectionRef,
    TransferBranchId,
};

/// The destination selected when one named disposition branch emits a transfer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferBranchBinding {
    compartment: CompartmentId,
    substance: SubstanceId,
    branch: TransferBranchId,
    destination: CompartmentId,
}

impl TransferBranchBinding {
    #[must_use]
    pub fn new(
        compartment: CompartmentId,
        substance: SubstanceId,
        branch: TransferBranchId,
        destination: CompartmentId,
    ) -> Self {
        Self {
            compartment,
            substance,
            branch,
            destination,
        }
    }
    #[must_use]
    pub fn compartment(&self) -> &CompartmentId {
        &self.compartment
    }
    #[must_use]
    pub fn substance(&self) -> &SubstanceId {
        &self.substance
    }
    #[must_use]
    pub fn branch(&self) -> &TransferBranchId {
        &self.branch
    }
    #[must_use]
    pub fn destination(&self) -> &CompartmentId {
        &self.destination
    }
}

/// Concise synonym used by callers that already establish branch context.
pub type TransferBinding = TransferBranchBinding;

/// A declared immutable source for a generic rule input leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleInputSource {
    /// A scalar value at the current timestep from a forcing series.
    Forcing(ForcingRef),
    /// A scalar value supplied through a declared interpolation table.
    InterpolationTable(InterpolatedTableRef),
    /// A typed value from a deterministic projection view.
    Projection(ProjectionRef),
}

impl RuleInputSource {
    #[must_use]
    pub fn value_kind(&self) -> ExpressionValueKind {
        match self {
            Self::Forcing(_) | Self::InterpolationTable(_) => ExpressionValueKind::Scalar,
            Self::Projection(reference) => match reference.value_kind() {
                crate::rule_reference::ProjectionValueKind::Extensive
                | crate::rule_reference::ProjectionValueKind::Scalar => ExpressionValueKind::Scalar,
                crate::rule_reference::ProjectionValueKind::Truth => ExpressionValueKind::Truth,
            },
        }
    }
}

/// The source selected for one input leaf of one compartment-substance rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleInputBinding {
    compartment: CompartmentId,
    substance: SubstanceId,
    reference: InputRef,
    source: RuleInputSource,
}

impl RuleInputBinding {
    #[must_use]
    pub fn new(
        compartment: CompartmentId,
        substance: SubstanceId,
        reference: InputRef,
        source: RuleInputSource,
    ) -> Self {
        Self {
            compartment,
            substance,
            reference,
            source,
        }
    }
    #[must_use]
    pub fn compartment(&self) -> &CompartmentId {
        &self.compartment
    }
    #[must_use]
    pub fn substance(&self) -> &SubstanceId {
        &self.substance
    }
    #[must_use]
    pub fn reference(&self) -> &InputRef {
        &self.reference
    }
    #[must_use]
    pub fn input(&self) -> &InputId {
        self.reference.id()
    }
    #[must_use]
    pub fn source(&self) -> &RuleInputSource {
        &self.source
    }
}

/// Canonically ordered execution bindings owned by a model artifact.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExecutionBindings {
    transfers: Vec<TransferBranchBinding>,
    inputs: Vec<RuleInputBinding>,
}

impl ExecutionBindings {
    /// Constructs bindings and rejects repeated rule-coordinate keys.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionBindingsError`] when a branch or input is bound more than once.
    pub fn new(
        transfers: impl IntoIterator<Item = TransferBranchBinding>,
        inputs: impl IntoIterator<Item = RuleInputBinding>,
    ) -> Result<Self, ExecutionBindingsError> {
        let mut transfers = transfers.into_iter().collect::<Vec<_>>();
        transfers.sort_by(|a, b| {
            (&a.compartment, &a.substance, &a.branch).cmp(&(
                &b.compartment,
                &b.substance,
                &b.branch,
            ))
        });
        let mut transfer_keys = BTreeSet::new();
        for binding in &transfers {
            let key = (
                binding.compartment.clone(),
                binding.substance.clone(),
                binding.branch.clone(),
            );
            if !transfer_keys.insert(key) {
                return Err(ExecutionBindingsError::DuplicateTransferBranch {
                    compartment: binding.compartment.clone(),
                    substance: binding.substance.clone(),
                    branch: binding.branch.clone(),
                });
            }
        }
        let mut inputs = inputs.into_iter().collect::<Vec<_>>();
        inputs.sort_by(|a, b| {
            (&a.compartment, &a.substance, a.reference.id()).cmp(&(
                &b.compartment,
                &b.substance,
                b.reference.id(),
            ))
        });
        let mut input_keys = BTreeSet::new();
        for binding in &inputs {
            let key = (
                binding.compartment.clone(),
                binding.substance.clone(),
                binding.reference.id().clone(),
            );
            if !input_keys.insert(key) {
                return Err(ExecutionBindingsError::DuplicateRuleInput {
                    compartment: binding.compartment.clone(),
                    substance: binding.substance.clone(),
                    input: binding.reference.id().clone(),
                });
            }
        }
        Ok(Self { transfers, inputs })
    }

    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn transfer_bindings(&self) -> impl ExactSizeIterator<Item = &TransferBranchBinding> {
        self.transfers.iter()
    }
    pub fn input_bindings(&self) -> impl ExactSizeIterator<Item = &RuleInputBinding> {
        self.inputs.iter()
    }

    #[must_use]
    pub fn destination(
        &self,
        compartment: &CompartmentId,
        substance: &SubstanceId,
        branch: &TransferBranchId,
    ) -> Option<&CompartmentId> {
        self.transfers
            .iter()
            .find(|binding| {
                binding.compartment() == compartment
                    && binding.substance() == substance
                    && binding.branch() == branch
            })
            .map(TransferBranchBinding::destination)
    }

    #[must_use]
    pub fn input_source(
        &self,
        compartment: &CompartmentId,
        substance: &SubstanceId,
        input: &InputId,
    ) -> Option<&RuleInputSource> {
        self.input_binding(compartment, substance, input)
            .map(RuleInputBinding::source)
    }

    #[must_use]
    pub(crate) fn input_binding(
        &self,
        compartment: &CompartmentId,
        substance: &SubstanceId,
        input: &InputId,
    ) -> Option<&RuleInputBinding> {
        self.inputs.iter().find(|binding| {
            binding.compartment() == compartment
                && binding.substance() == substance
                && binding.input() == input
        })
    }
}

/// Reports a repeated execution-binding coordinate.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum ExecutionBindingsError {
    /// Fires when one transfer branch has multiple destinations.
    #[error(
        "duplicate transfer binding for compartment `{compartment}`, substance `{substance}`, branch `{branch}`"
    )]
    DuplicateTransferBranch {
        compartment: CompartmentId,
        substance: SubstanceId,
        branch: TransferBranchId,
    },
    /// Fires when one rule input has multiple sources.
    #[error(
        "duplicate input binding for compartment `{compartment}`, substance `{substance}`, input `{input}`"
    )]
    DuplicateRuleInput {
        compartment: CompartmentId,
        substance: SubstanceId,
        input: InputId,
    },
}

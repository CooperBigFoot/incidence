//! projection_set : Ordered ProjectionSpec × InitialProjectorState ⇀ ValidatedDeterministicProjectionDAG   (pure, deterministic)

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::identity::{CompartmentId, SubstanceId};
use crate::numerical_semantics::{NumericalSemanticsError, NumericalSemanticsVersion};
use crate::rule_expression::{RuleExpr, RuleExprView};
pub use crate::rule_reference::ProjectionValueKind;
use crate::rule_reference::{
    ExpressionValueKind, InputId, InputRef, ParameterId, ParameterRef, ProjectionId, ProjectionRef,
};
use crate::versions::RuleIrVersion;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionBound {
    BoundedLagSteps,
    RollingWindow,
    RecurrenceStateSize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ForbiddenRecurrenceLeaf {
    Forcing,
    InterpolatedTable,
}

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum ProjectionError {
    /// Fires when a required projection bound is zero.
    #[error("projection {projection} has non-positive {bound:?} bound {attempted}")]
    NonPositiveBound {
        projection: ProjectionId,
        bound: ProjectionBound,
        attempted: usize,
    },
    /// Fires when an initial numeric value is NaN or infinite.
    #[error("projection {projection} initial value {index} rejects non-finite bits {bits:#018x}")]
    NonFiniteInitialValue {
        projection: ProjectionId,
        index: usize,
        bits: u64,
    },
    /// Fires when a projection identity is declared more than once.
    #[error("duplicate projection {projection}")]
    DuplicateProjection { projection: ProjectionId },
    /// Fires when initial state is declared more than once for a projection.
    #[error("duplicate initial state for projection {projection}")]
    DuplicateInitialState { projection: ProjectionId },
    /// Fires when a projection has no initial state declaration.
    #[error("missing initial state for projection {projection}")]
    MissingInitialState { projection: ProjectionId },
    /// Fires when initial state names no declared projection.
    #[error("unknown initial state for projection {projection}")]
    UnknownInitialState { projection: ProjectionId },
    /// Fires when initial state has the wrong number of values.
    #[error("projection {projection} initial state expected {expected} values, got {actual}")]
    InitialStateLength {
        projection: ProjectionId,
        expected: usize,
        actual: usize,
    },
    /// Fires when an initial-state slot has the wrong value kind.
    #[error("projection {projection} initial value {index} expected {expected:?}, got {actual:?}")]
    InitialStateKind {
        projection: ProjectionId,
        index: usize,
        expected: ProjectionValueKind,
        actual: ProjectionValueKind,
    },
    /// Fires when a projection refers to an undeclared projection.
    #[error("projection {owner} refers to unknown projection {reference}")]
    UnknownProjectionReference {
        owner: ProjectionId,
        reference: ProjectionId,
    },
    /// Fires when a projection reference declares the wrong value kind.
    #[error("projection {owner} reference {reference} expected {expected:?}, got {actual:?}")]
    ProjectionReferenceKind {
        owner: ProjectionId,
        reference: ProjectionId,
        expected: ProjectionValueKind,
        actual: ProjectionValueKind,
    },
    /// Fires when projection dependencies form a directed cycle.
    #[error("cyclic projection dependency {cycle:?}")]
    CyclicDependency { cycle: Vec<ProjectionId> },
    /// Fires when a dependency is not earlier in authored evaluation order.
    #[error("projection {owner} at {owner_index} depends on {reference} at {reference_index}")]
    DependencyNotEarlier {
        owner: ProjectionId,
        reference: ProjectionId,
        owner_index: usize,
        reference_index: usize,
    },
    /// Fires when a recurrence declares an input binding more than once.
    #[error("projection {projection} has duplicate input binding {input}")]
    DuplicateInputBinding {
        projection: ProjectionId,
        input: InputId,
    },
    /// Fires when a recurrence input leaf or declaration has no counterpart.
    #[error("projection {projection} has unmatched input binding {input}")]
    MissingInputBinding {
        projection: ProjectionId,
        input: InputId,
    },
    /// Fires when a recurrence input leaf and binding have incompatible kinds.
    #[error("projection {projection} input {input} expected {expected:?}, got {actual:?}")]
    InputBindingKind {
        projection: ProjectionId,
        input: InputId,
        expected: ExpressionValueKind,
        actual: ExpressionValueKind,
    },
    /// Fires when a prior-state binding indexes outside the recurrence state.
    #[error(
        "projection {projection} input {input} state index {attempted} is outside length {state_len}"
    )]
    PreviousStateIndex {
        projection: ProjectionId,
        input: InputId,
        state_len: usize,
        attempted: usize,
    },
    /// Fires when a prior-state binding declares the wrong slot kind.
    #[error(
        "projection {projection} input {input} state {index} expected {expected:?}, got {actual:?}"
    )]
    PreviousStateKind {
        projection: ProjectionId,
        input: InputId,
        index: usize,
        expected: ProjectionValueKind,
        actual: ProjectionValueKind,
    },
    /// Fires when a recurrence declares a parameter more than once.
    #[error("projection {projection} has duplicate parameter declaration {parameter}")]
    DuplicateParameterDeclaration {
        projection: ProjectionId,
        parameter: ParameterId,
    },
    /// Fires when a recurrence parameter leaf or declaration has no counterpart.
    #[error("projection {projection} has unmatched parameter declaration {parameter}")]
    MissingParameterDeclaration {
        projection: ProjectionId,
        parameter: ParameterId,
    },
    /// Fires when a recurrence parameter leaf and declaration have incompatible kinds.
    #[error("projection {projection} parameter {parameter} expected {expected:?}, got {actual:?}")]
    ParameterDeclarationKind {
        projection: ProjectionId,
        parameter: ParameterId,
        expected: ExpressionValueKind,
        actual: ExpressionValueKind,
    },
    /// Fires when a recurrence contains a leaf outside its authority contract.
    #[error("projection {projection} contains forbidden recurrence leaf {leaf:?}")]
    ForbiddenRecurrenceLeaf {
        projection: ProjectionId,
        leaf: ForbiddenRecurrenceLeaf,
    },
    /// Fires when recurrence update count differs from state size.
    #[error("projection {projection} expected {expected} recurrence updates, got {actual}")]
    RecurrenceUpdateCount {
        projection: ProjectionId,
        expected: usize,
        actual: usize,
    },
    /// Fires when a recurrence update has the wrong result kind for its slot.
    #[error("projection {projection} update {index} expected {expected:?}, got {actual:?}")]
    RecurrenceUpdateKind {
        projection: ProjectionId,
        index: usize,
        expected: ProjectionValueKind,
        actual: ExpressionValueKind,
    },
    /// Fires when the recurrence output slot is outside its state.
    #[error("projection {projection} output index {attempted} is outside state length {state_len}")]
    RecurrenceOutputIndex {
        projection: ProjectionId,
        state_len: usize,
        attempted: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoritativeFactSelector {
    IncomingTransferAmount {
        compartment: CompartmentId,
        substance: SubstanceId,
    },
    OutgoingTransferAmount {
        compartment: CompartmentId,
        substance: SubstanceId,
    },
}
impl AuthoritativeFactSelector {
    pub fn value_kind(&self) -> ProjectionValueKind {
        ProjectionValueKind::Extensive
    }
    pub fn compartment(&self) -> &CompartmentId {
        match self {
            Self::IncomingTransferAmount { compartment, .. }
            | Self::OutgoingTransferAmount { compartment, .. } => compartment,
        }
    }
    pub fn substance(&self) -> &SubstanceId {
        match self {
            Self::IncomingTransferAmount { substance, .. }
            | Self::OutgoingTransferAmount { substance, .. } => substance,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SelectorRef<'a> {
    IncomingTransferAmount {
        compartment: &'a str,
        substance: &'a str,
    },
    OutgoingTransferAmount {
        compartment: &'a str,
        substance: &'a str,
    },
}
impl Serialize for AuthoritativeFactSelector {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::IncomingTransferAmount {
                compartment,
                substance,
            } => SelectorRef::IncomingTransferAmount {
                compartment: compartment.as_str(),
                substance: substance.as_str(),
            },
            Self::OutgoingTransferAmount {
                compartment,
                substance,
            } => SelectorRef::OutgoingTransferAmount {
                compartment: compartment.as_str(),
                substance: substance.as_str(),
            },
        }
        .serialize(serializer)
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum SelectorWire {
    IncomingTransferAmount {
        compartment: String,
        substance: String,
    },
    OutgoingTransferAmount {
        compartment: String,
        substance: String,
    },
}
impl<'de> Deserialize<'de> for AuthoritativeFactSelector {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match SelectorWire::deserialize(deserializer)? {
            SelectorWire::IncomingTransferAmount {
                compartment,
                substance,
            } => Ok(Self::IncomingTransferAmount {
                compartment: CompartmentId::parse(&compartment)
                    .map_err(serde::de::Error::custom)?,
                substance: SubstanceId::parse(&substance).map_err(serde::de::Error::custom)?,
            }),
            SelectorWire::OutgoingTransferAmount {
                compartment,
                substance,
            } => Ok(Self::OutgoingTransferAmount {
                compartment: CompartmentId::parse(&compartment)
                    .map_err(serde::de::Error::custom)?,
                substance: SubstanceId::parse(&substance).map_err(serde::de::Error::custom)?,
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionSource {
    AuthoritativeFact(AuthoritativeFactSelector),
    Projection(ProjectionRef),
}
impl ProjectionSource {
    pub fn value_kind(&self) -> ProjectionValueKind {
        match self {
            Self::AuthoritativeFact(selector) => selector.value_kind(),
            Self::Projection(reference) => reference.value_kind(),
        }
    }
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ProjectionSourceRef<'a> {
    AuthoritativeFact {
        selector: &'a AuthoritativeFactSelector,
    },
    Projection {
        reference: &'a ProjectionRef,
    },
}
impl Serialize for ProjectionSource {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::AuthoritativeFact(selector) => {
                ProjectionSourceRef::AuthoritativeFact { selector }
            }
            Self::Projection(reference) => ProjectionSourceRef::Projection { reference },
        }
        .serialize(serializer)
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ProjectionSourceWire {
    AuthoritativeFact { selector: AuthoritativeFactSelector },
    Projection { reference: ProjectionRef },
}
impl<'de> Deserialize<'de> for ProjectionSource {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match ProjectionSourceWire::deserialize(deserializer)? {
            ProjectionSourceWire::AuthoritativeFact { selector } => {
                Self::AuthoritativeFact(selector)
            }
            ProjectionSourceWire::Projection { reference } => Self::Projection(reference),
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RollingAggregate {
    SumOldestToNewest,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InitialProjectionValue {
    Extensive(f64),
    Scalar(f64),
    Truth(bool),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiniteProjectionValue {
    bits: u64,
}
impl FiniteProjectionValue {
    /// Constructs a canonical finite projector value.
    ///
    /// # Errors
    ///
    /// Returns [`NumericalSemanticsError`] when `value` is NaN or infinite.
    pub fn new(
        semantics: NumericalSemanticsVersion,
        value: f64,
    ) -> Result<Self, NumericalSemanticsError> {
        semantics
            .normalize(value)
            .map(|v| Self { bits: v.to_bits() })
    }
    pub fn value(self) -> f64 {
        f64::from_bits(self.bits)
    }
    pub fn bits(self) -> u64 {
        self.bits
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionValue {
    Extensive(FiniteProjectionValue),
    Scalar(FiniteProjectionValue),
    Truth(bool),
}
impl ProjectionValue {
    pub fn value_kind(self) -> ProjectionValueKind {
        match self {
            Self::Extensive(_) => ProjectionValueKind::Extensive,
            Self::Scalar(_) => ProjectionValueKind::Scalar,
            Self::Truth(_) => ProjectionValueKind::Truth,
        }
    }
    pub fn value(self) -> Option<f64> {
        match self {
            Self::Extensive(v) | Self::Scalar(v) => Some(v.value()),
            Self::Truth(_) => None,
        }
    }
    pub fn bits(self) -> Option<u64> {
        match self {
            Self::Extensive(v) | Self::Scalar(v) => Some(v.bits()),
            Self::Truth(_) => None,
        }
    }
    pub fn truth(self) -> Option<bool> {
        match self {
            Self::Truth(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialProjectorState {
    projection: ProjectionId,
    values: Vec<ProjectionValue>,
}
impl InitialProjectorState {
    /// Converts raw initial values into canonical stored values.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionError::NonFiniteInitialValue`] for the first non-finite numeric value
    /// in ascending slot order.
    pub fn new(
        projection: ProjectionId,
        semantics: NumericalSemanticsVersion,
        values: Vec<InitialProjectionValue>,
    ) -> Result<Self, ProjectionError> {
        let mut stored = Vec::with_capacity(values.len());
        for (index, value) in values.into_iter().enumerate() {
            stored.push(match value {
                InitialProjectionValue::Extensive(v) => {
                    ProjectionValue::Extensive(FiniteProjectionValue::new(semantics, v).map_err(
                        |_| ProjectionError::NonFiniteInitialValue {
                            projection: projection.clone(),
                            index,
                            bits: v.to_bits(),
                        },
                    )?)
                }
                InitialProjectionValue::Scalar(v) => {
                    ProjectionValue::Scalar(FiniteProjectionValue::new(semantics, v).map_err(
                        |_| ProjectionError::NonFiniteInitialValue {
                            projection: projection.clone(),
                            index,
                            bits: v.to_bits(),
                        },
                    )?)
                }
                InitialProjectionValue::Truth(v) => ProjectionValue::Truth(v),
            });
        }
        Ok(Self {
            projection,
            values: stored,
        })
    }
    pub fn projection(&self) -> &ProjectionId {
        &self.projection
    }
    pub fn values(&self) -> &[ProjectionValue] {
        &self.values
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecurrenceInputSource {
    AuthoritativeFact(AuthoritativeFactSelector),
    PreviousState {
        index: usize,
        value_kind: ProjectionValueKind,
    },
}
impl RecurrenceInputSource {
    fn projection_kind(&self) -> ProjectionValueKind {
        match self {
            Self::AuthoritativeFact(_) => ProjectionValueKind::Extensive,
            Self::PreviousState { value_kind, .. } => *value_kind,
        }
    }
    fn expression_kind(&self) -> ExpressionValueKind {
        expression_kind(self.projection_kind())
    }
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RecurrenceInputSourceRef<'a> {
    AuthoritativeFact {
        selector: &'a AuthoritativeFactSelector,
    },
    PreviousState {
        index: usize,
        value_kind: ProjectionValueKind,
    },
}
impl Serialize for RecurrenceInputSource {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::AuthoritativeFact(selector) => {
                RecurrenceInputSourceRef::AuthoritativeFact { selector }
            }
            Self::PreviousState { index, value_kind } => RecurrenceInputSourceRef::PreviousState {
                index: *index,
                value_kind: *value_kind,
            },
        }
        .serialize(serializer)
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RecurrenceInputSourceWire {
    AuthoritativeFact {
        selector: AuthoritativeFactSelector,
    },
    PreviousState {
        index: usize,
        value_kind: ProjectionValueKind,
    },
}
impl<'de> Deserialize<'de> for RecurrenceInputSource {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match RecurrenceInputSourceWire::deserialize(deserializer)? {
                RecurrenceInputSourceWire::AuthoritativeFact { selector } => {
                    Self::AuthoritativeFact(selector)
                }
                RecurrenceInputSourceWire::PreviousState { index, value_kind } => {
                    Self::PreviousState { index, value_kind }
                }
            },
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecurrenceInputBinding {
    reference: InputRef,
    source: RecurrenceInputSource,
}
impl RecurrenceInputBinding {
    pub fn new(reference: InputRef, source: RecurrenceInputSource) -> Self {
        Self { reference, source }
    }
    pub fn reference(&self) -> &InputRef {
        &self.reference
    }
    pub fn input_source(&self) -> &RecurrenceInputSource {
        &self.source
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedLagSpec {
    rule_ir: RuleIrVersion,
    semantics: NumericalSemanticsVersion,
    id: ProjectionId,
    source: ProjectionSource,
    steps: usize,
}
impl BoundedLagSpec {
    /// Constructs a positive bounded-lag specification.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionError::NonPositiveBound`] when `steps` is zero.
    pub fn new(
        rule_ir: RuleIrVersion,
        semantics: NumericalSemanticsVersion,
        id: ProjectionId,
        source: ProjectionSource,
        steps: usize,
    ) -> Result<Self, ProjectionError> {
        if steps == 0 {
            return Err(ProjectionError::NonPositiveBound {
                projection: id,
                bound: ProjectionBound::BoundedLagSteps,
                attempted: steps,
            });
        }
        Ok(Self {
            rule_ir,
            semantics,
            id,
            source,
            steps,
        })
    }
    pub fn id(&self) -> &ProjectionId {
        &self.id
    }
    pub fn rule_ir_version(&self) -> RuleIrVersion {
        self.rule_ir
    }
    pub fn numerical_semantics_version(&self) -> NumericalSemanticsVersion {
        self.semantics
    }
    pub fn source(&self) -> &ProjectionSource {
        &self.source
    }
    pub fn steps(&self) -> usize {
        self.steps
    }
    pub fn value_kind(&self) -> ProjectionValueKind {
        self.source.value_kind()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderedRollingAggregateSpec {
    rule_ir: RuleIrVersion,
    semantics: NumericalSemanticsVersion,
    id: ProjectionId,
    source: ProjectionSource,
    window: usize,
    aggregate: RollingAggregate,
}
impl OrderedRollingAggregateSpec {
    /// Constructs a positive ordered rolling-aggregate specification.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionError::NonPositiveBound`] when `window` is zero.
    pub fn new(
        rule_ir: RuleIrVersion,
        semantics: NumericalSemanticsVersion,
        id: ProjectionId,
        source: ProjectionSource,
        window: usize,
        aggregate: RollingAggregate,
    ) -> Result<Self, ProjectionError> {
        if window == 0 {
            return Err(ProjectionError::NonPositiveBound {
                projection: id,
                bound: ProjectionBound::RollingWindow,
                attempted: window,
            });
        }
        Ok(Self {
            rule_ir,
            semantics,
            id,
            source,
            window,
            aggregate,
        })
    }
    pub fn id(&self) -> &ProjectionId {
        &self.id
    }
    pub fn rule_ir_version(&self) -> RuleIrVersion {
        self.rule_ir
    }
    pub fn numerical_semantics_version(&self) -> NumericalSemanticsVersion {
        self.semantics
    }
    pub fn source(&self) -> &ProjectionSource {
        &self.source
    }
    pub fn window(&self) -> usize {
        self.window
    }
    pub fn aggregate(&self) -> RollingAggregate {
        self.aggregate
    }
    pub fn value_kind(&self) -> ProjectionValueKind {
        self.source.value_kind()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteRecurrenceSpec {
    rule_ir: RuleIrVersion,
    semantics: NumericalSemanticsVersion,
    id: ProjectionId,
    state_kinds: Vec<ProjectionValueKind>,
    inputs: Vec<RecurrenceInputBinding>,
    parameters: Vec<ParameterRef>,
    updates: Vec<RuleExpr>,
    output_index: usize,
    dependencies: Vec<ProjectionRef>,
}
impl FiniteRecurrenceSpec {
    /// Constructs and validates a closed finite recurrence.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionError`] at the first deterministic structural, binding, leaf, kind,
    /// or version failure.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rule_ir: RuleIrVersion,
        semantics: NumericalSemanticsVersion,
        id: ProjectionId,
        state_kinds: Vec<ProjectionValueKind>,
        mut inputs: Vec<RecurrenceInputBinding>,
        mut parameters: Vec<ParameterRef>,
        updates: Vec<RuleExpr>,
        output_index: usize,
    ) -> Result<Self, ProjectionError> {
        if state_kinds.is_empty() {
            return Err(ProjectionError::NonPositiveBound {
                projection: id,
                bound: ProjectionBound::RecurrenceStateSize,
                attempted: 0,
            });
        }
        if updates.len() != state_kinds.len() {
            return Err(ProjectionError::RecurrenceUpdateCount {
                projection: id,
                expected: state_kinds.len(),
                actual: updates.len(),
            });
        }
        if output_index >= state_kinds.len() {
            return Err(ProjectionError::RecurrenceOutputIndex {
                projection: id,
                state_len: state_kinds.len(),
                attempted: output_index,
            });
        }
        inputs.sort_by(|a, b| a.reference.id().cmp(b.reference.id()));
        for pair in inputs.windows(2) {
            if pair[0].reference.id() == pair[1].reference.id() {
                return Err(ProjectionError::DuplicateInputBinding {
                    projection: id,
                    input: pair[1].reference.id().clone(),
                });
            }
        }
        parameters.sort_by(|a, b| a.id().cmp(b.id()));
        for pair in parameters.windows(2) {
            if pair[0].id() == pair[1].id() {
                return Err(ProjectionError::DuplicateParameterDeclaration {
                    projection: id,
                    parameter: pair[1].id().clone(),
                });
            }
        }
        for binding in &inputs {
            if let RecurrenceInputSource::PreviousState { index, value_kind } =
                binding.input_source()
            {
                if *index >= state_kinds.len() {
                    return Err(ProjectionError::PreviousStateIndex {
                        projection: id,
                        input: binding.reference.id().clone(),
                        state_len: state_kinds.len(),
                        attempted: *index,
                    });
                }
                if state_kinds[*index] != *value_kind {
                    return Err(ProjectionError::PreviousStateKind {
                        projection: id,
                        input: binding.reference.id().clone(),
                        index: *index,
                        expected: state_kinds[*index],
                        actual: *value_kind,
                    });
                }
            }
        }
        let mut reached_inputs = BTreeSet::new();
        let mut reached_parameters = BTreeSet::new();
        let mut dependencies = Vec::new();
        for (index, update) in updates.iter().enumerate() {
            let expected = state_kinds[index];
            if expression_kind(expected) != update.value_kind() {
                return Err(ProjectionError::RecurrenceUpdateKind {
                    projection: id,
                    index,
                    expected,
                    actual: update.value_kind(),
                });
            }
            walk_update(
                &id,
                update,
                &inputs,
                &parameters,
                &mut reached_inputs,
                &mut reached_parameters,
                &mut dependencies,
            )?;
        }
        for binding in &inputs {
            if !reached_inputs.contains(binding.reference.id()) {
                return Err(ProjectionError::MissingInputBinding {
                    projection: id,
                    input: binding.reference.id().clone(),
                });
            }
        }
        for parameter in &parameters {
            if !reached_parameters.contains(parameter.id()) {
                return Err(ProjectionError::MissingParameterDeclaration {
                    projection: id,
                    parameter: parameter.id().clone(),
                });
            }
        }
        Ok(Self {
            rule_ir,
            semantics,
            id,
            state_kinds,
            inputs,
            parameters,
            updates,
            output_index,
            dependencies,
        })
    }
    pub fn id(&self) -> &ProjectionId {
        &self.id
    }
    pub fn rule_ir_version(&self) -> RuleIrVersion {
        self.rule_ir
    }
    pub fn numerical_semantics_version(&self) -> NumericalSemanticsVersion {
        self.semantics
    }
    pub fn state_kinds(&self) -> &[ProjectionValueKind] {
        &self.state_kinds
    }
    pub fn inputs(&self) -> &[RecurrenceInputBinding] {
        &self.inputs
    }
    pub fn parameters(&self) -> &[ParameterRef] {
        &self.parameters
    }
    pub fn updates(&self) -> &[RuleExpr] {
        &self.updates
    }
    pub fn output_index(&self) -> usize {
        self.output_index
    }
    pub fn value_kind(&self) -> ProjectionValueKind {
        self.state_kinds[self.output_index]
    }
    pub fn dependencies(&self) -> impl Iterator<Item = &ProjectionRef> {
        self.dependencies.iter()
    }
}

fn expression_kind(kind: ProjectionValueKind) -> ExpressionValueKind {
    match kind {
        ProjectionValueKind::Extensive | ProjectionValueKind::Scalar => ExpressionValueKind::Scalar,
        ProjectionValueKind::Truth => ExpressionValueKind::Truth,
    }
}

fn walk_update(
    projection: &ProjectionId,
    expr: &RuleExpr,
    inputs: &[RecurrenceInputBinding],
    parameters: &[ParameterRef],
    reached_inputs: &mut BTreeSet<InputId>,
    reached_parameters: &mut BTreeSet<ParameterId>,
    dependencies: &mut Vec<ProjectionRef>,
) -> Result<(), ProjectionError> {
    match expr.view() {
        RuleExprView::Input(reference) => {
            let binding = inputs
                .binary_search_by(|b| b.reference.id().cmp(reference.id()))
                .ok()
                .map(|i| &inputs[i])
                .ok_or_else(|| ProjectionError::MissingInputBinding {
                    projection: projection.clone(),
                    input: reference.id().clone(),
                })?;
            let actual = binding.source.expression_kind();
            if reference.value_kind() != actual {
                return Err(ProjectionError::InputBindingKind {
                    projection: projection.clone(),
                    input: reference.id().clone(),
                    expected: reference.value_kind(),
                    actual,
                });
            }
            reached_inputs.insert(reference.id().clone());
        }
        RuleExprView::Parameter(reference) => {
            let declaration = parameters
                .binary_search_by(|p| p.id().cmp(reference.id()))
                .ok()
                .map(|i| &parameters[i])
                .ok_or_else(|| ProjectionError::MissingParameterDeclaration {
                    projection: projection.clone(),
                    parameter: reference.id().clone(),
                })?;
            if reference.value_kind() != declaration.value_kind() {
                return Err(ProjectionError::ParameterDeclarationKind {
                    projection: projection.clone(),
                    parameter: reference.id().clone(),
                    expected: reference.value_kind(),
                    actual: declaration.value_kind(),
                });
            }
            reached_parameters.insert(reference.id().clone());
        }
        RuleExprView::Forcing(_) => {
            return Err(ProjectionError::ForbiddenRecurrenceLeaf {
                projection: projection.clone(),
                leaf: ForbiddenRecurrenceLeaf::Forcing,
            });
        }
        RuleExprView::Projection(reference) => dependencies.push(reference.clone()),
        RuleExprView::Literal(_) => {}
        RuleExprView::InterpolatedTable { .. } => {
            return Err(ProjectionError::ForbiddenRecurrenceLeaf {
                projection: projection.clone(),
                leaf: ForbiddenRecurrenceLeaf::InterpolatedTable,
            });
        }
        RuleExprView::Add { lhs, rhs }
        | RuleExprView::Subtract { lhs, rhs }
        | RuleExprView::Multiply { lhs, rhs }
        | RuleExprView::Divide { lhs, rhs }
        | RuleExprView::Minimum { lhs, rhs }
        | RuleExprView::Maximum { lhs, rhs }
        | RuleExprView::Comparison { lhs, rhs, .. } => {
            walk_update(
                projection,
                lhs,
                inputs,
                parameters,
                reached_inputs,
                reached_parameters,
                dependencies,
            )?;
            walk_update(
                projection,
                rhs,
                inputs,
                parameters,
                reached_inputs,
                reached_parameters,
                dependencies,
            )?;
        }
        RuleExprView::Clamp {
            value,
            lower,
            upper,
        } => {
            for child in [value, lower, upper] {
                walk_update(
                    projection,
                    child,
                    inputs,
                    parameters,
                    reached_inputs,
                    reached_parameters,
                    dependencies,
                )?;
            }
        }
        RuleExprView::Select {
            condition,
            when_true,
            when_false,
        } => {
            for child in [condition, when_true, when_false] {
                walk_update(
                    projection,
                    child,
                    inputs,
                    parameters,
                    reached_inputs,
                    reached_parameters,
                    dependencies,
                )?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionSpec {
    BoundedLag(BoundedLagSpec),
    OrderedRollingAggregate(OrderedRollingAggregateSpec),
    FiniteRecurrence(FiniteRecurrenceSpec),
}
pub enum ProjectionSpecView<'a> {
    BoundedLag(&'a BoundedLagSpec),
    OrderedRollingAggregate(&'a OrderedRollingAggregateSpec),
    FiniteRecurrence(&'a FiniteRecurrenceSpec),
}
impl ProjectionSpec {
    pub fn id(&self) -> &ProjectionId {
        match self {
            Self::BoundedLag(v) => v.id(),
            Self::OrderedRollingAggregate(v) => v.id(),
            Self::FiniteRecurrence(v) => v.id(),
        }
    }
    pub fn rule_ir_version(&self) -> RuleIrVersion {
        match self {
            Self::BoundedLag(v) => v.rule_ir_version(),
            Self::OrderedRollingAggregate(v) => v.rule_ir_version(),
            Self::FiniteRecurrence(v) => v.rule_ir_version(),
        }
    }
    pub fn numerical_semantics_version(&self) -> NumericalSemanticsVersion {
        match self {
            Self::BoundedLag(v) => v.numerical_semantics_version(),
            Self::OrderedRollingAggregate(v) => v.numerical_semantics_version(),
            Self::FiniteRecurrence(v) => v.numerical_semantics_version(),
        }
    }
    pub fn value_kind(&self) -> ProjectionValueKind {
        match self {
            Self::BoundedLag(v) => v.value_kind(),
            Self::OrderedRollingAggregate(v) => v.value_kind(),
            Self::FiniteRecurrence(v) => v.value_kind(),
        }
    }
    pub fn view(&self) -> ProjectionSpecView<'_> {
        match self {
            Self::BoundedLag(v) => ProjectionSpecView::BoundedLag(v),
            Self::OrderedRollingAggregate(v) => ProjectionSpecView::OrderedRollingAggregate(v),
            Self::FiniteRecurrence(v) => ProjectionSpecView::FiniteRecurrence(v),
        }
    }
    pub fn dependencies(&self) -> Vec<&ProjectionRef> {
        match self {
            Self::BoundedLag(v) => match v.source() {
                ProjectionSource::Projection(reference) => vec![reference],
                _ => vec![],
            },
            Self::OrderedRollingAggregate(v) => match v.source() {
                ProjectionSource::Projection(reference) => vec![reference],
                _ => vec![],
            },
            Self::FiniteRecurrence(v) => v.dependencies().collect(),
        }
    }
}
impl From<BoundedLagSpec> for ProjectionSpec {
    fn from(v: BoundedLagSpec) -> Self {
        Self::BoundedLag(v)
    }
}
impl From<OrderedRollingAggregateSpec> for ProjectionSpec {
    fn from(v: OrderedRollingAggregateSpec) -> Self {
        Self::OrderedRollingAggregate(v)
    }
}
impl From<FiniteRecurrenceSpec> for ProjectionSpec {
    fn from(v: FiniteRecurrenceSpec) -> Self {
        Self::FiniteRecurrence(v)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionSet {
    specifications: Vec<ProjectionSpec>,
    initial_states: BTreeMap<ProjectionId, InitialProjectorState>,
}
impl ProjectionSet {
    /// Validates projection identities, dependencies, order, and initial-state shapes.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionError`] at the first deterministic identity, dependency, order, or
    /// initial-state failure.
    pub fn new(
        specifications: Vec<ProjectionSpec>,
        states: Vec<InitialProjectorState>,
    ) -> Result<Self, ProjectionError> {
        let mut catalogue = BTreeMap::new();
        for (index, spec) in specifications.iter().enumerate() {
            if catalogue
                .insert(spec.id().clone(), (spec.value_kind(), index))
                .is_some()
            {
                return Err(ProjectionError::DuplicateProjection {
                    projection: spec.id().clone(),
                });
            }
        }
        let mut initial_states = BTreeMap::new();
        for state in states {
            let id = state.projection().clone();
            if initial_states.insert(id.clone(), state).is_some() {
                return Err(ProjectionError::DuplicateInitialState { projection: id });
            }
        }
        for spec in &specifications {
            if !initial_states.contains_key(spec.id()) {
                return Err(ProjectionError::MissingInitialState {
                    projection: spec.id().clone(),
                });
            }
        }
        for id in initial_states.keys() {
            if !catalogue.contains_key(id) {
                return Err(ProjectionError::UnknownInitialState {
                    projection: id.clone(),
                });
            }
        }
        for spec in &specifications {
            for reference in spec.dependencies() {
                let Some((actual, _)) = catalogue.get(reference.id()) else {
                    return Err(ProjectionError::UnknownProjectionReference {
                        owner: spec.id().clone(),
                        reference: reference.id().clone(),
                    });
                };
                if reference.value_kind() != *actual {
                    return Err(ProjectionError::ProjectionReferenceKind {
                        owner: spec.id().clone(),
                        reference: reference.id().clone(),
                        expected: reference.value_kind(),
                        actual: *actual,
                    });
                }
            }
        }
        if let Some(cycle) = find_cycle(&specifications) {
            return Err(ProjectionError::CyclicDependency { cycle });
        }
        for (owner_index, spec) in specifications.iter().enumerate() {
            for reference in spec.dependencies() {
                let (_, reference_index) = catalogue.get(reference.id()).ok_or_else(|| {
                    ProjectionError::UnknownProjectionReference {
                        owner: spec.id().clone(),
                        reference: reference.id().clone(),
                    }
                })?;
                if *reference_index >= owner_index {
                    return Err(ProjectionError::DependencyNotEarlier {
                        owner: spec.id().clone(),
                        reference: reference.id().clone(),
                        owner_index,
                        reference_index: *reference_index,
                    });
                }
            }
        }
        for spec in &specifications {
            let state = initial_states.get(spec.id()).ok_or_else(|| {
                ProjectionError::MissingInitialState {
                    projection: spec.id().clone(),
                }
            })?;
            let expected_len = match spec {
                ProjectionSpec::BoundedLag(v) => v.steps(),
                ProjectionSpec::OrderedRollingAggregate(v) => v
                    .window()
                    .checked_sub(1)
                    .ok_or_else(|| ProjectionError::NonPositiveBound {
                        projection: v.id().clone(),
                        bound: ProjectionBound::RollingWindow,
                        attempted: v.window(),
                    })?,
                ProjectionSpec::FiniteRecurrence(v) => v.state_kinds().len(),
            };
            if state.values().len() != expected_len {
                return Err(ProjectionError::InitialStateLength {
                    projection: spec.id().clone(),
                    expected: expected_len,
                    actual: state.values().len(),
                });
            }
            for (index, value) in state.values().iter().enumerate() {
                let expected = match spec {
                    ProjectionSpec::BoundedLag(v) => v.value_kind(),
                    ProjectionSpec::OrderedRollingAggregate(v) => v.value_kind(),
                    ProjectionSpec::FiniteRecurrence(v) => v.state_kinds()[index],
                };
                if value.value_kind() != expected {
                    return Err(ProjectionError::InitialStateKind {
                        projection: spec.id().clone(),
                        index,
                        expected,
                        actual: value.value_kind(),
                    });
                }
            }
        }
        Ok(Self {
            specifications,
            initial_states,
        })
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ProjectionSpec> {
        self.specifications.iter()
    }
    pub fn initial_states(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ProjectionId, &InitialProjectorState)> {
        self.initial_states.iter()
    }
    pub fn initial_state(&self, projection: &ProjectionId) -> Option<&InitialProjectorState> {
        self.initial_states.get(projection)
    }
}

fn find_cycle(specifications: &[ProjectionSpec]) -> Option<Vec<ProjectionId>> {
    let by_id: BTreeMap<_, _> = specifications.iter().map(|s| (s.id().clone(), s)).collect();
    fn visit(
        id: &ProjectionId,
        by_id: &BTreeMap<ProjectionId, &ProjectionSpec>,
        active: &mut Vec<ProjectionId>,
        done: &mut BTreeSet<ProjectionId>,
    ) -> Option<Vec<ProjectionId>> {
        if let Some(position) = active.iter().position(|v| v == id) {
            let mut cycle = active[position..].to_vec();
            cycle.push(id.clone());
            return Some(cycle);
        }
        if done.contains(id) {
            return None;
        }
        active.push(id.clone());
        if let Some(spec) = by_id.get(id) {
            for dependency in spec.dependencies() {
                if by_id.contains_key(dependency.id())
                    && let Some(cycle) = visit(dependency.id(), by_id, active, done)
                {
                    return Some(cycle);
                }
            }
        }
        active.pop();
        done.insert(id.clone());
        None
    }
    let mut done = BTreeSet::new();
    for id in by_id.keys() {
        if let Some(cycle) = visit(id, &by_id, &mut Vec::new(), &mut done) {
            return Some(cycle);
        }
    }
    None
}

// Serde uses constructor reconstruction so invariant-bearing values cannot be decoded unchecked.
#[derive(Serialize)]
struct InitialStateRef<'a> {
    projection: &'a ProjectionId,
    values: Vec<ValueRef>,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ValueRef {
    Extensive { value: f64 },
    Scalar { value: f64 },
    Truth { value: bool },
}
impl Serialize for InitialProjectorState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        InitialStateRef {
            projection: &self.projection,
            values: self
                .values
                .iter()
                .map(|v| match v {
                    ProjectionValue::Extensive(n) => ValueRef::Extensive { value: n.value() },
                    ProjectionValue::Scalar(n) => ValueRef::Scalar { value: n.value() },
                    ProjectionValue::Truth(value) => ValueRef::Truth { value: *value },
                })
                .collect(),
        }
        .serialize(serializer)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InitialStateWire {
    projection: ProjectionId,
    values: Vec<ValueWire>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ValueWire {
    Extensive { value: f64 },
    Scalar { value: f64 },
    Truth { value: bool },
}
impl<'de> Deserialize<'de> for InitialProjectorState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = InitialStateWire::deserialize(deserializer)?;
        InitialProjectorState::new(
            wire.projection,
            NumericalSemanticsVersion::V1,
            wire.values
                .into_iter()
                .map(|v| match v {
                    ValueWire::Extensive { value } => InitialProjectionValue::Extensive(value),
                    ValueWire::Scalar { value } => InitialProjectionValue::Scalar(value),
                    ValueWire::Truth { value } => InitialProjectionValue::Truth(value),
                })
                .collect(),
        )
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Serialize)]
struct SpecRef<'a> {
    rule_ir_version: VersionWire,
    numerical_semantics_version: VersionWire,
    id: &'a ProjectionId,
    value_kind: ProjectionValueKind,
    spec: FamilyRef<'a>,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum VersionWire {
    V1,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FamilyRef<'a> {
    BoundedLag {
        source: &'a ProjectionSource,
        steps: usize,
    },
    OrderedRollingAggregate {
        source: &'a ProjectionSource,
        window: usize,
        aggregate: RollingAggregate,
    },
    FiniteRecurrence {
        state_kinds: &'a [ProjectionValueKind],
        inputs: &'a [RecurrenceInputBinding],
        parameters: &'a [ParameterRef],
        updates: &'a [RuleExpr],
        output_index: usize,
    },
}
impl Serialize for ProjectionSpec {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let spec = match self {
            Self::BoundedLag(v) => FamilyRef::BoundedLag {
                source: &v.source,
                steps: v.steps,
            },
            Self::OrderedRollingAggregate(v) => FamilyRef::OrderedRollingAggregate {
                source: &v.source,
                window: v.window,
                aggregate: v.aggregate,
            },
            Self::FiniteRecurrence(v) => FamilyRef::FiniteRecurrence {
                state_kinds: &v.state_kinds,
                inputs: &v.inputs,
                parameters: &v.parameters,
                updates: &v.updates,
                output_index: v.output_index,
            },
        };
        SpecRef {
            rule_ir_version: VersionWire::V1,
            numerical_semantics_version: VersionWire::V1,
            id: self.id(),
            value_kind: self.value_kind(),
            spec,
        }
        .serialize(serializer)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpecWire {
    rule_ir_version: VersionWire,
    numerical_semantics_version: VersionWire,
    id: ProjectionId,
    value_kind: ProjectionValueKind,
    spec: FamilyWire,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum FamilyWire {
    BoundedLag {
        source: ProjectionSource,
        steps: usize,
    },
    OrderedRollingAggregate {
        source: ProjectionSource,
        window: usize,
        aggregate: RollingAggregate,
    },
    FiniteRecurrence {
        state_kinds: Vec<ProjectionValueKind>,
        inputs: Vec<RecurrenceInputBinding>,
        parameters: Vec<ParameterRef>,
        updates: Vec<RuleExpr>,
        output_index: usize,
    },
}
impl<'de> Deserialize<'de> for ProjectionSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = SpecWire::deserialize(deserializer)?;
        let _ = (wire.rule_ir_version, wire.numerical_semantics_version);
        let result: Result<ProjectionSpec, ProjectionError> = match wire.spec {
            FamilyWire::BoundedLag { source, steps } => BoundedLagSpec::new(
                RuleIrVersion::V1,
                NumericalSemanticsVersion::V1,
                wire.id,
                source,
                steps,
            )
            .map(Into::into),
            FamilyWire::OrderedRollingAggregate {
                source,
                window,
                aggregate,
            } => OrderedRollingAggregateSpec::new(
                RuleIrVersion::V1,
                NumericalSemanticsVersion::V1,
                wire.id,
                source,
                window,
                aggregate,
            )
            .map(Into::into),
            FamilyWire::FiniteRecurrence {
                state_kinds,
                inputs,
                parameters,
                updates,
                output_index,
            } => FiniteRecurrenceSpec::new(
                RuleIrVersion::V1,
                NumericalSemanticsVersion::V1,
                wire.id,
                state_kinds,
                inputs,
                parameters,
                updates,
                output_index,
            )
            .map(Into::into),
        };
        let spec = result.map_err(serde::de::Error::custom)?;
        if wire.value_kind != spec.value_kind() {
            return Err(serde::de::Error::custom(format_args!(
                "projection {} declared value kind {:?} does not match derived {:?}",
                spec.id(),
                wire.value_kind,
                spec.value_kind()
            )));
        }
        Ok(spec)
    }
}

#[derive(Serialize)]
struct SetRef<'a> {
    specifications: &'a [ProjectionSpec],
    initial_states: Vec<&'a InitialProjectorState>,
}
impl Serialize for ProjectionSet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SetRef {
            specifications: &self.specifications,
            initial_states: self.initial_states.values().collect(),
        }
        .serialize(serializer)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetWire {
    specifications: Vec<ProjectionSpec>,
    initial_states: Vec<InitialProjectorState>,
}
impl<'de> Deserialize<'de> for ProjectionSet {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = SetWire::deserialize(deserializer)?;
        Self::new(wire.specifications, wire.initial_states).map_err(serde::de::Error::custom)
    }
}

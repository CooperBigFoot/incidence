//! execution : ModelArtifact × AuthoritativeLogPrefix → AuthoritativeLog   (deterministic)
//!
//! The executor interprets the closed rule IR in calendar and topology order.  Its only mutation
//! of authoritative state is [`ValidatedTransaction::commit`]; projector values are disposable
//! and are rebuilt from Genesis and transfer facts whenever a prefix is resumed.

use std::collections::BTreeMap;

use crate::disposition::{
    Allocation, Disposition, SubstanceDisposition, TransactionError, ValidatedTransaction,
};
use crate::execution_bindings::RuleInputSource;
use crate::identity::{CompartmentId, SubstanceId};
use crate::interpolation_table::{InterpolationBoundaryPolicy, InterpolationTable};
use crate::ledger::{
    AuthoritativeLog, LogError, ReplayError, RunId, Transfer, TransferEndpoint,
    replay_with_artifact,
};
use crate::model_artifact::{ModelArtifact, RuleDefinition};
use crate::non_negative_amount::{NonNegativeAmount, NonNegativeAmountError};
use crate::numerical_semantics::{NumericalSemanticsError, ScalarComparison};
use crate::partition_expression::PartitionExprView;
use crate::presence::ValueState;
use crate::projection::{
    AuthoritativeFactSelector, ProjectionSource, ProjectionSpecView, ProjectionValue,
    RecurrenceInputSource,
};
use crate::rule_expression::{RuleExpr, RuleExprView};
use crate::rule_reference::{
    ForcingId, InputId, ParameterId, ProjectionId, ProjectionRef, TableId, TransferBranchId,
};
use crate::temporal::TimestepIndex;
use crate::topology::TopologyEndpoint;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Value {
    Scalar(f64),
    Truth(bool),
}
impl Value {
    fn scalar(self) -> Result<f64, ExecutionError> {
        match self {
            Self::Scalar(v) => Ok(v),
            Self::Truth(_) => Err(ExecutionError::ValueKind),
        }
    }
    fn truth(self) -> Result<bool, ExecutionError> {
        match self {
            Self::Truth(v) => Ok(v),
            Self::Scalar(_) => Err(ExecutionError::ValueKind),
        }
    }
}

/// Failures while interpreting or executing an immutable artifact.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ExecutionError {
    /// Fires when the supplied prefix is incompatible with the immutable artifact.
    #[error("authoritative prefix cannot be resumed: {source}")]
    Replay {
        #[from]
        source: ReplayError,
    },
    /// Fires when an already completed log is passed to a continuation operation.
    #[error("completed authoritative log cannot be resumed")]
    AlreadyCompleted,
    /// Fires when a supplied transfer differs from deterministic execution at the same record index.
    #[error("authoritative prefix diverges from deterministic execution at transfer {index}")]
    PrefixDivergence { index: usize },
    /// Fires when a supplied prefix contains more transfers than deterministic execution.
    #[error(
        "authoritative prefix has {actual} transfers, but deterministic execution has only {expected}"
    )]
    PrefixTooLong { expected: usize, actual: usize },
    /// Fires when an expression resolves a source that is unexpectedly absent.
    #[error(
        "rule for compartment `{compartment}`, substance `{substance}` cannot resolve {kind} `{identity}` at timestep {timestep:?}"
    )]
    MissingValue {
        compartment: CompartmentId,
        substance: SubstanceId,
        timestep: TimestepIndex,
        kind: &'static str,
        identity: String,
    },
    /// Fires when a typed expression value is used as the other closed value kind.
    #[error("interpreter encountered an incompatible typed value")]
    ValueKind,
    /// Fires when the selected numerical protocol rejects an operation.
    #[error("numerical interpretation failed: {source}")]
    Numerical {
        #[from]
        source: NumericalSemanticsError,
    },
    /// Fires when a scalar transfer amount is negative or otherwise not an amount.
    #[error(
        "rule for compartment `{compartment}`, substance `{substance}` produced invalid amount bits {bits:#018x} at timestep {timestep:?}"
    )]
    InvalidAmount {
        compartment: CompartmentId,
        substance: SubstanceId,
        timestep: TimestepIndex,
        bits: u64,
    },
    /// Fires when an evaluated transfer exceeds the stock visible at its topological turn.
    #[error(
        "rule for compartment `{compartment}`, substance `{substance}` requests {requested_bits:#018x} from available {available_bits:#018x} at timestep {timestep:?}"
    )]
    RuleOverdraw {
        compartment: CompartmentId,
        substance: SubstanceId,
        timestep: TimestepIndex,
        available_bits: u64,
        requested_bits: u64,
    },
    /// Fires when the validated atomic write boundary rejects a disposition.
    #[error("validated disposition commit failed: {source}")]
    Transaction {
        #[from]
        source: TransactionError,
    },
    /// Fires when the completion seal cannot be appended.
    #[error("authoritative completion seal failed: {source}")]
    Seal { source: LogError },
    /// Fires when a projection has no initial state despite artifact validation.
    #[error("projection `{projection}` has no initial state")]
    MissingProjectionState { projection: ProjectionId },
    /// Fires when a projection cannot represent its declared typed value.
    #[error("projection `{projection}` has an incompatible state value")]
    ProjectionKind { projection: ProjectionId },
    /// Fires when a bare rule input is bound to a table (tables require an explicit lookup expression).
    #[error("rule input `{input}` is bound to table `{table}` without a lookup abscissa")]
    TableInputNeedsAbscissa { input: InputId, table: TableId },
    /// Fires when interpolation rejects an input under the table's boundary policy.
    #[error("table `{table}` rejects input bits {input_bits:#018x}")]
    TableBoundary { table: TableId, input_bits: u64 },
    /// Fires when arithmetic cannot advance beyond the represented timestep.
    #[error("timestep coordinate overflows after {timestep:?}")]
    TimestepOverflow { timestep: TimestepIndex },
}

/// The closed V1 rule-IR interpreter.
pub struct RuleInterpreter<'a> {
    artifact: &'a ModelArtifact,
    log: &'a AuthoritativeLog,
    rule: &'a RuleDefinition,
    timestep: TimestepIndex,
}
impl<'a> RuleInterpreter<'a> {
    #[must_use]
    pub fn new(
        artifact: &'a ModelArtifact,
        log: &'a AuthoritativeLog,
        rule: &'a RuleDefinition,
        timestep: TimestepIndex,
    ) -> Self {
        Self {
            artifact,
            log,
            rule,
            timestep,
        }
    }
    /// Evaluates the rule's scalar expression under the artifact's selected semantics.
    pub fn evaluate(&self) -> Result<f64, ExecutionError> {
        self.evaluate_expression(self.rule.expression())
    }
    fn evaluate_expression(&self, expression: &RuleExpr) -> Result<f64, ExecutionError> {
        self.eval(expression, None, None)?.scalar()
    }
    fn missing(&self, kind: &'static str, identity: String) -> ExecutionError {
        ExecutionError::MissingValue {
            compartment: self.rule.compartment().clone(),
            substance: self.rule.substance().clone(),
            timestep: self.timestep,
            kind,
            identity,
        }
    }
    fn eval(
        &self,
        expr: &RuleExpr,
        inputs: Option<&BTreeMap<InputId, Value>>,
        state_params: Option<&BTreeMap<ParameterId, f64>>,
    ) -> Result<Value, ExecutionError> {
        let s = expr.numerical_semantics_version();
        Ok(match expr.view() {
            RuleExprView::Input(r) => {
                if let Some(v) = inputs.and_then(|m| m.get(r.id())).copied() {
                    v
                } else {
                    self.eval_bound_input(r.id())?
                }
            }
            RuleExprView::Parameter(r) => Value::Scalar(
                state_params
                    .and_then(|m| m.get(r.id()))
                    .copied()
                    .or_else(|| {
                        self.rule
                            .parameters()
                            .find(|(id, _)| *id == r.id())
                            .map(|(_, v)| v)
                    })
                    .ok_or_else(|| self.missing("parameter", r.id().as_str().to_owned()))?,
            ),
            RuleExprView::Forcing(r) => Value::Scalar(self.forcing(r.id())?),
            RuleExprView::Projection(r) => self.projection(r)?,
            RuleExprView::Literal(v) => Value::Scalar(v.value()),
            RuleExprView::Add { lhs, rhs } => Value::Scalar(s.add(
                self.eval(lhs, inputs, state_params)?.scalar()?,
                self.eval(rhs, inputs, state_params)?.scalar()?,
            )?),
            RuleExprView::Subtract { lhs, rhs } => Value::Scalar(s.subtract(
                self.eval(lhs, inputs, state_params)?.scalar()?,
                self.eval(rhs, inputs, state_params)?.scalar()?,
            )?),
            RuleExprView::Multiply { lhs, rhs } => Value::Scalar(s.multiply(
                self.eval(lhs, inputs, state_params)?.scalar()?,
                self.eval(rhs, inputs, state_params)?.scalar()?,
            )?),
            RuleExprView::Divide { lhs, rhs } => Value::Scalar(s.divide(
                self.eval(lhs, inputs, state_params)?.scalar()?,
                self.eval(rhs, inputs, state_params)?.scalar()?,
            )?),
            RuleExprView::Power { lhs, rhs } => Value::Scalar(s.power(
                self.eval(lhs, inputs, state_params)?.scalar()?,
                self.eval(rhs, inputs, state_params)?.scalar()?,
            )?),
            RuleExprView::Minimum { lhs, rhs } => Value::Scalar(s.minimum(
                self.eval(lhs, inputs, state_params)?.scalar()?,
                self.eval(rhs, inputs, state_params)?.scalar()?,
            )?),
            RuleExprView::Maximum { lhs, rhs } => Value::Scalar(s.maximum(
                self.eval(lhs, inputs, state_params)?.scalar()?,
                self.eval(rhs, inputs, state_params)?.scalar()?,
            )?),
            RuleExprView::Clamp {
                value,
                lower,
                upper,
            } => Value::Scalar(s.clamp(
                self.eval(value, inputs, state_params)?.scalar()?,
                self.eval(lower, inputs, state_params)?.scalar()?,
                self.eval(upper, inputs, state_params)?.scalar()?,
            )?),
            RuleExprView::Comparison {
                comparison,
                lhs,
                rhs,
            } => Value::Truth(s.compare(
                comparison,
                self.eval(lhs, inputs, state_params)?.scalar()?,
                self.eval(rhs, inputs, state_params)?.scalar()?,
            )?),
            RuleExprView::Select {
                condition,
                when_true,
                when_false,
            } => {
                if self.eval(condition, inputs, state_params)?.truth()? {
                    self.eval(when_true, inputs, state_params)?
                } else {
                    self.eval(when_false, inputs, state_params)?
                }
            }
            RuleExprView::InterpolatedTable { table, input } => Value::Scalar(self.interpolate(
                table.id(),
                self.eval(input, inputs, state_params)?.scalar()?,
            )?),
        })
    }
    fn forcing(&self, id: &ForcingId) -> Result<f64, ExecutionError> {
        self.artifact
            .forcings()
            .find(|x| x.id() == id)
            .and_then(|x| match x.value_at(self.timestep) {
                ValueState::Present(v) => Some(v),
                _ => None,
            })
            .ok_or_else(|| self.missing("forcing", id.as_str().to_owned()))
    }
    fn eval_bound_input(&self, id: &InputId) -> Result<Value, ExecutionError> {
        match self
            .artifact
            .rule_input_source(self.rule.compartment(), self.rule.substance(), id)
            .ok_or_else(|| self.missing("input", id.as_str().to_owned()))?
        {
            RuleInputSource::Forcing(r) => Ok(Value::Scalar(self.forcing(r.id())?)),
            RuleInputSource::Projection(r) => self.projection(r),
            RuleInputSource::InterpolationTable(r) => {
                Err(ExecutionError::TableInputNeedsAbscissa {
                    input: id.clone(),
                    table: r.id().clone(),
                })
            }
        }
    }
    fn interpolate(&self, id: &TableId, x: f64) -> Result<f64, ExecutionError> {
        let t = self
            .artifact
            .tables()
            .find(|t| t.id() == id)
            .ok_or_else(|| self.missing("table", id.as_str().to_owned()))?;
        interpolate(t, x)
    }
    fn projection(&self, r: &ProjectionRef) -> Result<Value, ExecutionError> {
        self.projection_at(r.id(), self.timestep)
    }
    fn projection_at(&self, id: &ProjectionId, t: TimestepIndex) -> Result<Value, ExecutionError> {
        let spec = self
            .artifact
            .projections()
            .iter()
            .find(|x| x.id() == id)
            .ok_or_else(|| self.missing("projection", id.as_str().to_owned()))?;
        let initial = self
            .artifact
            .projections()
            .initial_state(id)
            .ok_or_else(|| ExecutionError::MissingProjectionState {
                projection: id.clone(),
            })?;
        match spec.view() {
            ProjectionSpecView::BoundedLag(lag) => {
                let first = self.artifact.horizon().first().value();
                let n = lag.steps() as u64;
                if t.value() < first + n {
                    projection_value(initial.values()[(t.value() - first) as usize], id)
                } else {
                    self.projection_source(lag.source(), TimestepIndex::new(t.value() - n))
                }
            }
            ProjectionSpecView::OrderedRollingAggregate(rolling) => {
                let first = self.artifact.horizon().first().value();
                let observed = t.value().saturating_sub(first).saturating_add(1);
                let observed_in_window = observed.min(rolling.window() as u64);
                let start = t
                    .value()
                    .saturating_add(1)
                    .saturating_sub(observed_in_window);
                let missing = rolling.window().saturating_sub(observed_in_window as usize);
                let mut total = 0.0;
                for value in initial
                    .values()
                    .iter()
                    .skip(initial.values().len() - missing)
                {
                    total = rolling
                        .numerical_semantics_version()
                        .add(total, projection_value(*value, id)?.scalar()?)?;
                }
                for ordinal in start..=t.value() {
                    total = rolling.numerical_semantics_version().add(
                        total,
                        self.projection_source(rolling.source(), TimestepIndex::new(ordinal))?
                            .scalar()?,
                    )?;
                }
                Ok(Value::Scalar(total))
            }
            ProjectionSpecView::FiniteRecurrence(rec) => {
                let mut state = initial
                    .values()
                    .iter()
                    .map(|v| projection_value(*v, id))
                    .collect::<Result<Vec<_>, _>>()?;
                let params = self
                    .rule
                    .parameters()
                    .map(|(id, v)| (id.clone(), v))
                    .collect::<BTreeMap<_, _>>();
                for ordinal in self.artifact.horizon().first().value()..=t.value() {
                    let step = TimestepIndex::new(ordinal);
                    let mut bound = BTreeMap::new();
                    for b in rec.inputs() {
                        let value = match b.input_source() {
                            RecurrenceInputSource::AuthoritativeFact(sel) => {
                                Value::Scalar(self.fact(sel, step)?)
                            }
                            RecurrenceInputSource::PreviousState { index, .. } => state[*index],
                        };
                        bound.insert(b.reference().id().clone(), value);
                    }
                    let nested = Self {
                        artifact: self.artifact,
                        log: self.log,
                        rule: self.rule,
                        timestep: step,
                    };
                    state = rec
                        .updates()
                        .iter()
                        .map(|e| nested.eval(e, Some(&bound), Some(&params)))
                        .collect::<Result<Vec<_>, _>>()?;
                }
                Ok(state[rec.output_index()])
            }
        }
    }
    fn projection_source(
        &self,
        source: &ProjectionSource,
        t: TimestepIndex,
    ) -> Result<Value, ExecutionError> {
        match source {
            ProjectionSource::AuthoritativeFact(sel) => Ok(Value::Scalar(self.fact(sel, t)?)),
            ProjectionSource::Projection(r) => self.projection_at(r.id(), t),
        }
    }
    fn fact(
        &self,
        selector: &AuthoritativeFactSelector,
        t: TimestepIndex,
    ) -> Result<f64, ExecutionError> {
        let s = self.artifact.versions().numerical_semantics();
        let mut total = 0.0;
        for transfer in self
            .log
            .transfers()
            .iter()
            .filter(|x| x.timestep() == t && selector_matches(selector, x))
        {
            let a = match transfer.amounts().amount(selector.substance()) {
                ValueState::Present(v) => v.value(),
                _ => {
                    return Err(self.missing(
                        "authoritative fact substance",
                        selector.substance().as_str().to_owned(),
                    ));
                }
            };
            total = s.add(total, a)?;
        }
        Ok(total)
    }
}

fn projection_value(value: ProjectionValue, _id: &ProjectionId) -> Result<Value, ExecutionError> {
    match value {
        ProjectionValue::Extensive(v) | ProjectionValue::Scalar(v) => Ok(Value::Scalar(v.value())),
        ProjectionValue::Truth(v) => Ok(Value::Truth(v)),
    }
}
fn selector_matches(selector: &AuthoritativeFactSelector, t: &Transfer) -> bool {
    match selector {
        AuthoritativeFactSelector::IncomingTransferAmount { compartment, .. } => {
            t.target().id() == compartment
        }
        AuthoritativeFactSelector::OutgoingTransferAmount { compartment, .. } => {
            t.source().id() == compartment
        }
    }
}
fn interpolate(table: &InterpolationTable, x: f64) -> Result<f64, ExecutionError> {
    let points = table.points().collect::<Vec<_>>();
    let first = points[0];
    let last = points[points.len() - 1];
    if x < first.0 {
        return match table.boundary_policy() {
            InterpolationBoundaryPolicy::Reject => Err(ExecutionError::TableBoundary {
                table: table.id().clone(),
                input_bits: x.to_bits(),
            }),
            InterpolationBoundaryPolicy::ClampToEndpoint => Ok(first.1),
            InterpolationBoundaryPolicy::LinearExtrapolate => table
                .numerical_semantics_version()
                .interpolate_linear(x, first.0, points[1].0, first.1, points[1].1)
                .map_err(Into::into),
        };
    }
    if x > last.0 {
        return match table.boundary_policy() {
            InterpolationBoundaryPolicy::Reject => Err(ExecutionError::TableBoundary {
                table: table.id().clone(),
                input_bits: x.to_bits(),
            }),
            InterpolationBoundaryPolicy::ClampToEndpoint => Ok(last.1),
            InterpolationBoundaryPolicy::LinearExtrapolate => {
                let p = points[points.len() - 2];
                table
                    .numerical_semantics_version()
                    .interpolate_linear(x, p.0, last.0, p.1, last.1)
                    .map_err(Into::into)
            }
        };
    }
    if x == last.0 {
        return Ok(last.1);
    };
    let pair = points
        .windows(2)
        .find(|p| x >= p[0].0 && x <= p[1].0)
        .ok_or_else(|| ExecutionError::TableBoundary {
            table: table.id().clone(),
            input_bits: x.to_bits(),
        })?;
    if x == pair[0].0 {
        Ok(pair[0].1)
    } else {
        table
            .numerical_semantics_version()
            .interpolate_linear(x, pair[0].0, pair[1].0, pair[0].1, pair[1].1)
            .map_err(Into::into)
    }
}

/// Executes or resumes immutable artifacts in their authored calendar and topological order.
pub enum StepExecutor {}
impl StepExecutor {
    /// Runs a new model and seals its authoritative log.
    pub fn run(
        artifact: &ModelArtifact,
        run_id: RunId,
    ) -> Result<AuthoritativeLog, ExecutionError> {
        Self::generate(artifact, run_id)
    }

    /// Continues a valid unsealed record prefix and appends a completion seal.
    ///
    /// Continuation deliberately regenerates disposable state from Genesis and compares the
    /// supplied transfers in record order.  Consequently a prefix may end between two transfers
    /// of one topological turn without carrying a hidden program counter.
    pub fn resume(
        artifact: &ModelArtifact,
        log: &mut AuthoritativeLog,
    ) -> Result<(), ExecutionError> {
        replay_with_artifact(log, artifact)?;
        if log.is_sealed() {
            return Err(ExecutionError::AlreadyCompleted);
        }
        let generated = Self::generate(artifact, log.genesis().run_id())?;
        if log.transfer_count() > generated.transfer_count() {
            return Err(ExecutionError::PrefixTooLong {
                expected: generated.transfer_count(),
                actual: log.transfer_count(),
            });
        }
        for (index, (actual, expected)) in log
            .transfers()
            .iter()
            .zip(generated.transfers())
            .enumerate()
        {
            if actual != expected {
                return Err(ExecutionError::PrefixDivergence { index });
            }
        }
        *log = generated;
        Ok(())
    }

    fn generate(
        artifact: &ModelArtifact,
        run_id: RunId,
    ) -> Result<AuthoritativeLog, ExecutionError> {
        let mut log = AuthoritativeLog::for_run(run_id, artifact);
        let mut ordinal = artifact.horizon().first().value();
        loop {
            let timestep = TimestepIndex::new(ordinal);
            Self::execute_timestep(artifact, &mut log, timestep)?;
            if timestep == artifact.horizon().last() {
                break;
            }
            ordinal = ordinal
                .checked_add(1)
                .ok_or(ExecutionError::TimestepOverflow { timestep })?;
        }
        log.seal(artifact.horizon().last())
            .map_err(|source| ExecutionError::Seal { source })?;
        Ok(log)
    }

    fn execute_timestep(
        artifact: &ModelArtifact,
        log: &mut AuthoritativeLog,
        timestep: TimestepIndex,
    ) -> Result<(), ExecutionError> {
        for compartment in artifact.topology().topological_order() {
            let Some(TopologyEndpoint::Finite(finite)) = artifact.topology().endpoint(compartment)
            else {
                continue;
            };
            let replay = replay_with_artifact(log, artifact)?;
            let mut substances = Vec::new();
            for substance in artifact.registry().iter() {
                let available = match replay.final_state().finite_stock(compartment, substance) {
                    ValueState::Present(value) => value,
                    ValueState::Absent | ValueState::NotModelled => {
                        return Err(ExecutionError::MissingValue {
                            compartment: compartment.clone(),
                            substance: substance.clone(),
                            timestep,
                            kind: "stock",
                            identity: substance.as_str().to_owned(),
                        });
                    }
                };
                let rule = artifact.rules().find(|rule| {
                    rule.compartment() == compartment && rule.substance() == substance
                });
                let (retained, allocations) = if let Some(rule) = rule {
                    evaluate_partition(artifact, log, rule, timestep, available)?
                } else {
                    (available, Vec::new())
                };
                substances.push(SubstanceDisposition::new(
                    substance.clone(),
                    retained,
                    allocations,
                ));
            }
            ValidatedTransaction::commit(
                log,
                artifact,
                Disposition::new(timestep, finite.clone(), substances),
            )?;
        }
        Ok(())
    }
}

fn evaluate_partition(
    artifact: &ModelArtifact,
    log: &AuthoritativeLog,
    rule: &RuleDefinition,
    timestep: TimestepIndex,
    available: NonNegativeAmount,
) -> Result<(NonNegativeAmount, Vec<Allocation>), ExecutionError> {
    let interpreter = RuleInterpreter::new(artifact, log, rule, timestep);
    let s = artifact.versions().numerical_semantics();
    let mut allocations = Vec::new();
    let mut add_branch =
        |branch: &TransferBranchId, a: NonNegativeAmount| -> Result<(), ExecutionError> {
            let id = artifact
                .transfer_destination(rule.compartment(), rule.substance(), branch)
                .ok_or_else(|| ExecutionError::MissingValue {
                    compartment: rule.compartment().clone(),
                    substance: rule.substance().clone(),
                    timestep,
                    kind: "branch",
                    identity: branch.as_str().to_owned(),
                })?;
            let target = match artifact.topology().endpoint(id) {
                Some(TopologyEndpoint::Finite(v)) => TransferEndpoint::Finite(v.clone()),
                Some(TopologyEndpoint::Boundary(v)) => TransferEndpoint::Boundary(v.clone()),
                None => {
                    return Err(ExecutionError::MissingValue {
                        compartment: rule.compartment().clone(),
                        substance: rule.substance().clone(),
                        timestep,
                        kind: "endpoint",
                        identity: id.as_str().to_owned(),
                    });
                }
            };
            allocations.push(Allocation::new(target, a));
            Ok(())
        };
    let transfer_total = match rule.disposition().view() {
        PartitionExprView::RetainAll => {
            let _evaluated_amount = amount(rule, timestep, interpreter.evaluate()?)?;
            0.0
        }
        PartitionExprView::ReleaseAll { branch } => {
            let evaluated_amount = amount(rule, timestep, interpreter.evaluate()?)?;
            add_branch(branch, evaluated_amount)?;
            evaluated_amount.value()
        }
        PartitionExprView::ConstantFractionTransfer { branch, fraction } => {
            let evaluated_amount = amount(rule, timestep, interpreter.evaluate()?)?;
            let v = s.multiply(evaluated_amount.value(), fraction.value())?;
            let a = amount(rule, timestep, v)?;
            add_branch(branch, a)?;
            v
        }
        PartitionExprView::FixedFractionSplit {
            retained_fraction: _,
            branches,
        } => {
            let evaluated_amount = amount(rule, timestep, interpreter.evaluate()?)?;
            let mut total = 0.0;
            for b in branches {
                let v = s.multiply(evaluated_amount.value(), b.fraction().value())?;
                let a = amount(rule, timestep, v)?;
                add_branch(b.branch(), a)?;
                total = s.add(total, v)?;
            }
            total
        }
        PartitionExprView::ExogenousSeries { branch, series } => {
            let _evaluated_amount = amount(rule, timestep, interpreter.evaluate()?)?;
            let v = artifact
                .forcings()
                .find(|x| x.id() == series.id())
                .and_then(|x| match x.value_at(timestep) {
                    ValueState::Present(v) => Some(v),
                    _ => None,
                })
                .ok_or_else(|| ExecutionError::MissingValue {
                    compartment: rule.compartment().clone(),
                    substance: rule.substance().clone(),
                    timestep,
                    kind: "forcing",
                    identity: series.id().as_str().to_owned(),
                })?;
            let a = amount(rule, timestep, v)?;
            add_branch(branch, a)?;
            v
        }
        PartitionExprView::ExpressionPartition { branches } => {
            let mut total = 0.0;
            for branch in branches {
                let value = interpreter.evaluate_expression(branch.expression())?;
                let allocation = amount(rule, timestep, value)?;
                add_branch(branch.branch(), allocation)?;
                total = s.add(total, value)?;
            }
            total
        }
    };
    if s.compare(
        ScalarComparison::GreaterThan,
        transfer_total,
        available.value(),
    )? {
        return Err(ExecutionError::RuleOverdraw {
            compartment: rule.compartment().clone(),
            substance: rule.substance().clone(),
            timestep,
            available_bits: available.value().to_bits(),
            requested_bits: transfer_total.to_bits(),
        });
    }
    let retained_value = allocations
        .iter()
        .try_fold(available.value(), |remaining, allocation| {
            s.subtract(remaining, allocation.amount().value())
        })?;
    let retained = amount(rule, timestep, retained_value)?;
    Ok((retained, allocations))
}
fn amount(
    rule: &RuleDefinition,
    timestep: TimestepIndex,
    value: f64,
) -> Result<NonNegativeAmount, ExecutionError> {
    NonNegativeAmount::try_from(value).map_err(|_e: NonNegativeAmountError| {
        ExecutionError::InvalidAmount {
            compartment: rule.compartment().clone(),
            substance: rule.substance().clone(),
            timestep,
            bits: value.to_bits(),
        }
    })
}

/// Runs a new artifact using the topological executor.
pub fn execute_model(
    artifact: &ModelArtifact,
    run_id: RunId,
) -> Result<AuthoritativeLog, ExecutionError> {
    StepExecutor::run(artifact, run_id)
}
/// Resumes an unsealed authoritative prefix in place.
pub fn resume_from_prefix(
    artifact: &ModelArtifact,
    log: &mut AuthoritativeLog,
) -> Result<(), ExecutionError> {
    StepExecutor::resume(artifact, log)
}

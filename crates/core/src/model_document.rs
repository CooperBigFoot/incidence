//! model_document : PlainModelData ⇀ ModelArtifact   (pure, deterministic)
//!
//! This is the public whole-model wire form. It contains only serialisable data and delegates
//! every domain invariant to the same constructors used by native Rust callers.

use crate::endpoints::{BoundaryAccount, FiniteCompartment};
use crate::execution_bindings::{
    ExecutionBindings, RuleInputBinding, RuleInputSource, TransferBranchBinding,
};
use crate::forcing::ForcingSeries;
use crate::identity::{CompartmentId, SubstanceId};
use crate::initial_stocks::InitialStocks;
use crate::interpolation_table::InterpolationTable;
use crate::model_artifact::{
    ModelArtifact, ModelVersions, Quantum, RuleDefinition, SubstanceUnit, UnitId,
};
use crate::non_negative_amount::NonNegativeAmount;
use crate::numerical_semantics::NumericalSemanticsVersion;
use crate::partition_expression::PartitionExpr;
use crate::projection::ProjectionSet;
use crate::rule_expression::RuleExpr;
use crate::rule_reference::{
    ExpressionValueKind, ForcingId, ForcingRef, InputId, InputRef, InterpolatedTableRef,
    ParameterId, ProjectionId, ProjectionRef, ProjectionValueKind, TableId, TransferBranchId,
};
use crate::sparse_substance_vector::SparseSubstanceVector;
use crate::substance_registry::SubstanceRegistry;
use crate::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use crate::topology::{DirectedConnection, Topology, TopologyEndpoint};
use crate::versions::{CanonicalEncodingVersion, InterpreterVersion, RuleIrVersion};
use serde::{Deserialize, Serialize};

/// A complete plain-data model description.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDocument {
    pub document_version: ModelDocumentVersion,
    pub versions: ModelVersionsDocument,
    pub finite_compartments: Vec<String>,
    pub boundary_accounts: Vec<String>,
    pub connections: Vec<ConnectionDocument>,
    pub substances: Vec<String>,
    pub initial_stocks: Vec<InitialStockDocument>,
    pub calendar: CalendarDocument,
    pub horizon: HorizonDocument,
    pub projections: ProjectionSet,
    pub forcings: Vec<ForcingSeries>,
    pub interpolation_tables: Vec<InterpolationTable>,
    pub rules: Vec<RuleDocument>,
    pub transfer_bindings: Vec<TransferBindingDocument>,
    pub input_bindings: Vec<InputBindingDocument>,
    pub units: Vec<UnitDocument>,
}

/// Version of the authored model-document schema.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelDocumentVersion {
    /// The first public whole-model document schema.
    V1,
}

/// Protocol identities selected by a document.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelVersionsDocument {
    pub rule_ir: RuleIrVersion,
    pub interpreter: InterpreterVersion,
    pub numerical_semantics: NumericalSemanticsVersion,
    pub canonical_encoding: CanonicalEncodingVersion,
}

impl Default for ModelVersionsDocument {
    fn default() -> Self {
        Self {
            rule_ir: RuleIrVersion::V1,
            interpreter: InterpreterVersion::V1,
            numerical_semantics: NumericalSemanticsVersion::V1,
            canonical_encoding: CanonicalEncodingVersion::V1,
        }
    }
}

/// One directed topology edge.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionDocument {
    pub source: String,
    pub target: String,
}

/// One supplied compartment stock vector. Omitted registered substances are modelled zeros.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InitialStockDocument {
    pub compartment: String,
    pub amounts: Vec<SubstanceAmountDocument>,
}

/// One substance amount in an initial stock vector.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubstanceAmountDocument {
    pub substance: String,
    pub amount: f64,
}

/// Fixed-step calendar data.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarDocument {
    pub origin_unix_seconds: i64,
    pub timestep_seconds: u64,
}

/// Inclusive execution horizon data.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HorizonDocument {
    pub first: u64,
    pub last: u64,
}

/// One rule and its immutable declared parameters.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDocument {
    pub compartment: String,
    pub substance: String,
    pub expression: RuleExpr,
    pub disposition: PartitionExpr,
    pub parameters: Vec<ParameterDocument>,
}

/// One declared scalar rule parameter.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterDocument {
    pub id: String,
    pub value: f64,
}

/// One disposition branch destination.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransferBindingDocument {
    pub compartment: String,
    pub substance: String,
    pub branch: String,
    pub destination: String,
}

/// A plain-data source for a generic rule input.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputSourceDocument {
    Forcing {
        forcing: String,
    },
    InterpolationTable {
        table: String,
    },
    Projection {
        projection: String,
        value_kind: ProjectionValueKind,
    },
}

/// One generic rule-input binding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InputBindingDocument {
    pub compartment: String,
    pub substance: String,
    pub input: String,
    pub value_kind: ExpressionValueKind,
    pub source: InputSourceDocument,
}

/// The display unit and arithmetic quantum assigned to one registered substance.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnitDocument {
    pub substance: String,
    pub unit: String,
    pub quantum: f64,
}

impl<'de> Deserialize<'de> for UnitDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireUnitDocument {
            substance: String,
            unit: String,
            quantum: Option<f64>,
        }

        let wire = WireUnitDocument::deserialize(deserializer)?;
        let quantum = wire.quantum.ok_or_else(|| {
            serde::de::Error::custom("units component requires a `quantum` declaration")
        })?;
        Ok(Self {
            substance: wire.substance,
            unit: wire.unit,
            quantum,
        })
    }
}

/// Reports which document component failed domain parsing or validation.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
#[error("invalid model document {component}: {reason}")]
pub struct ModelDocumentError {
    pub component: &'static str,
    pub reason: String,
}

fn invalid(component: &'static str, error: impl std::fmt::Display) -> ModelDocumentError {
    ModelDocumentError {
        component,
        reason: error.to_string(),
    }
}
fn compartment(value: &str) -> Result<CompartmentId, ModelDocumentError> {
    CompartmentId::parse(value).map_err(|e| invalid("compartment identity", e))
}
fn substance(value: &str) -> Result<SubstanceId, ModelDocumentError> {
    SubstanceId::parse(value).map_err(|e| invalid("substance identity", e))
}

impl ModelDocument {
    /// Converts this wire value into the immutable executable artifact.
    ///
    /// # Errors
    ///
    /// Returns [`ModelDocumentError`] with the original domain diagnostic when any field or
    /// cross-component relationship is invalid.
    pub fn artifact(&self) -> Result<ModelArtifact, ModelDocumentError> {
        let endpoints = self
            .finite_compartments
            .iter()
            .map(|id| {
                compartment(id)
                    .map(FiniteCompartment::new)
                    .map(TopologyEndpoint::Finite)
            })
            .chain(self.boundary_accounts.iter().map(|id| {
                compartment(id)
                    .map(BoundaryAccount::new)
                    .map(TopologyEndpoint::Boundary)
            }))
            .collect::<Result<Vec<_>, _>>()?;
        let connections = self
            .connections
            .iter()
            .map(|edge| {
                Ok(DirectedConnection::new(
                    compartment(&edge.source)?,
                    compartment(&edge.target)?,
                ))
            })
            .collect::<Result<Vec<_>, ModelDocumentError>>()?;
        let topology = Topology::new(endpoints, connections).map_err(|e| invalid("topology", e))?;
        let registry = SubstanceRegistry::new(
            self.substances
                .iter()
                .map(|id| substance(id))
                .collect::<Result<Vec<_>, _>>()?,
        )
        .map_err(|e| invalid("substance registry", e))?;
        let stocks = self
            .initial_stocks
            .iter()
            .map(|stock| {
                let amounts = stock
                    .amounts
                    .iter()
                    .map(|entry| {
                        Ok((
                            substance(&entry.substance)?,
                            NonNegativeAmount::try_from(entry.amount)
                                .map_err(|e| invalid("initial stock amount", e))?,
                        ))
                    })
                    .collect::<Result<Vec<_>, ModelDocumentError>>()?;
                let vector = SparseSubstanceVector::new(&registry, amounts)
                    .map_err(|e| invalid("initial stock vector", e))?;
                Ok((compartment(&stock.compartment)?, vector))
            })
            .collect::<Result<Vec<_>, ModelDocumentError>>()?;
        let initial_stocks = InitialStocks::new(&topology, &registry, stocks)
            .map_err(|e| invalid("initial stocks", e))?;
        let duration = TimestepDuration::from_seconds(self.calendar.timestep_seconds)
            .map_err(|e| invalid("calendar", e))?;
        let calendar = FixedStepCalendar::new(
            CalendarOrigin::new(CalendarInstant::from_unix_seconds(
                self.calendar.origin_unix_seconds,
            )),
            duration,
        );
        let horizon = RunHorizon::new(
            TimestepIndex::new(self.horizon.first),
            TimestepIndex::new(self.horizon.last),
        )
        .map_err(|e| invalid("horizon", e))?;
        let rules = self
            .rules
            .iter()
            .map(|rule| {
                RuleDefinition::new(
                    compartment(&rule.compartment)?,
                    substance(&rule.substance)?,
                    rule.expression.clone(),
                    rule.disposition.clone(),
                    rule.parameters
                        .iter()
                        .map(|parameter| {
                            ParameterId::parse(&parameter.id)
                                .map(|id| (id, parameter.value))
                                .map_err(|e| invalid("rule parameter identity", e))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
                .map_err(|e| invalid("rule", e))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for stock in &self.initial_stocks {
            let stock_compartment = compartment(&stock.compartment)?;
            if !rules
                .iter()
                .any(|rule| rule.compartment() == &stock_compartment)
            {
                continue;
            }
            for entry in &stock.amounts {
                if entry.amount <= 0.0 {
                    continue;
                }
                let stock_substance = substance(&entry.substance)?;
                if !rules.iter().any(|rule| {
                    rule.compartment() == &stock_compartment && rule.substance() == &stock_substance
                }) {
                    return Err(ModelDocumentError {
                        component: "rule coverage",
                        reason: format!(
                            "rule for compartment `{stock_compartment}` omits substance `{stock_substance}` at timestep {}",
                            horizon.first().value()
                        ),
                    });
                }
            }
        }
        let transfers = self
            .transfer_bindings
            .iter()
            .map(|binding| {
                Ok(TransferBranchBinding::new(
                    compartment(&binding.compartment)?,
                    substance(&binding.substance)?,
                    TransferBranchId::parse(&binding.branch)
                        .map_err(|e| invalid("transfer branch identity", e))?,
                    compartment(&binding.destination)?,
                ))
            })
            .collect::<Result<Vec<_>, ModelDocumentError>>()?;
        let inputs = self
            .input_bindings
            .iter()
            .map(|binding| {
                let source = match &binding.source {
                    InputSourceDocument::Forcing { forcing } => {
                        RuleInputSource::Forcing(ForcingRef::new(
                            ForcingId::parse(forcing)
                                .map_err(|e| invalid("forcing identity", e))?,
                        ))
                    }
                    InputSourceDocument::InterpolationTable { table } => {
                        RuleInputSource::InterpolationTable(InterpolatedTableRef::new(
                            TableId::parse(table).map_err(|e| invalid("table identity", e))?,
                        ))
                    }
                    InputSourceDocument::Projection {
                        projection,
                        value_kind,
                    } => RuleInputSource::Projection(ProjectionRef::new(
                        ProjectionId::parse(projection)
                            .map_err(|e| invalid("projection identity", e))?,
                        *value_kind,
                    )),
                };
                Ok(RuleInputBinding::new(
                    compartment(&binding.compartment)?,
                    substance(&binding.substance)?,
                    InputRef::new(
                        InputId::parse(&binding.input).map_err(|e| invalid("input identity", e))?,
                        binding.value_kind,
                    ),
                    source,
                ))
            })
            .collect::<Result<Vec<_>, ModelDocumentError>>()?;
        let bindings = ExecutionBindings::new(transfers, inputs)
            .map_err(|e| invalid("execution bindings", e))?;
        let units = self
            .units
            .iter()
            .map(|unit| {
                Ok((
                    substance(&unit.substance)?,
                    SubstanceUnit::new(
                        UnitId::parse(&unit.unit).map_err(|e| invalid("units", e))?,
                        Quantum::try_from(unit.quantum).map_err(|e| invalid("units", e))?,
                    ),
                ))
            })
            .collect::<Result<Vec<_>, ModelDocumentError>>()?;
        let versions = ModelVersions::new(
            self.versions.rule_ir,
            self.versions.interpreter,
            self.versions.numerical_semantics,
            self.versions.canonical_encoding,
        );
        ModelArtifact::builder(topology, registry, initial_stocks, calendar, horizon)
            .with_projections(self.projections.clone())
            .with_forcings(self.forcings.clone())
            .with_tables(self.interpolation_tables.clone())
            .with_rules(rules)
            .with_execution_bindings(bindings)
            .with_units(units)
            .with_versions(versions)
            .build()
            .map_err(|e| invalid("artifact", e))
    }

    /// Consumes and compiles this document into an immutable executable artifact.
    ///
    /// # Errors
    ///
    /// Returns [`ModelDocumentError`] under the same conditions as [`Self::artifact`].
    pub fn into_artifact(self) -> Result<ModelArtifact, ModelDocumentError> {
        self.artifact()
    }
}

impl TryFrom<&ModelDocument> for ModelArtifact {
    type Error = ModelDocumentError;
    fn try_from(document: &ModelDocument) -> Result<Self, Self::Error> {
        document.artifact()
    }
}
impl TryFrom<ModelDocument> for ModelArtifact {
    type Error = ModelDocumentError;
    fn try_from(document: ModelDocument) -> Result<Self, Self::Error> {
        document.into_artifact()
    }
}

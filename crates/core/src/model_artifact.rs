//! model_artifact : ModelComponents ⇀ (ModelDigest, ImmutableModelArtifact)   (pure, deterministic)
//!
//! The digest is SHA-256 over the artifact's canonical V1 bytes.  Every value required to
//! reproduce execution is owned by the artifact; callers can observe, but cannot mutate, it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::canonical_encoding::{
    CanonicalEncode, CanonicalEncodingError, CanonicalField, CanonicalPayloadWriter,
};
use crate::forcing::ForcingSeries;
use crate::identity::{CompartmentId, SubstanceId};
use crate::initial_stocks::InitialStocks;
use crate::interpolation_table::InterpolationTable;
use crate::numerical_semantics::NumericalSemanticsVersion;
use crate::partition_expression::{PartitionExpr, PartitionExprView};
use crate::projection::ProjectionSet;
use crate::rule_expression::{RuleExpr, RuleExprView};
use crate::rule_reference::{ForcingId, ParameterId, ProjectionId, TableId};
use crate::substance_registry::SubstanceRegistry;
use crate::temporal::{FixedStepCalendar, RunHorizon};
use crate::topology::{Topology, TopologyEndpoint};
use crate::versions::{CanonicalEncodingVersion, InterpreterVersion, RuleIrVersion};

/// The protocol versions that participate in a model artifact's identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelVersions {
    rule_ir: RuleIrVersion,
    interpreter: InterpreterVersion,
    numerical_semantics: NumericalSemanticsVersion,
    canonical_encoding: CanonicalEncodingVersion,
}

impl ModelVersions {
    /// Constructs an explicit version selection.
    #[must_use]
    pub fn new(
        rule_ir: RuleIrVersion,
        interpreter: InterpreterVersion,
        numerical_semantics: NumericalSemanticsVersion,
        canonical_encoding: CanonicalEncodingVersion,
    ) -> Self {
        Self {
            rule_ir,
            interpreter,
            numerical_semantics,
            canonical_encoding,
        }
    }

    #[must_use]
    pub fn rule_ir(self) -> RuleIrVersion {
        self.rule_ir
    }
    #[must_use]
    pub fn interpreter(self) -> InterpreterVersion {
        self.interpreter
    }
    #[must_use]
    pub fn numerical_semantics(self) -> NumericalSemanticsVersion {
        self.numerical_semantics
    }
    #[must_use]
    pub fn canonical_encoding(self) -> CanonicalEncodingVersion {
        self.canonical_encoding
    }
}

impl Default for ModelVersions {
    fn default() -> Self {
        Self::new(
            RuleIrVersion::V1,
            InterpreterVersion::V1,
            NumericalSemanticsVersion::V1,
            CanonicalEncodingVersion::V1,
        )
    }
}

/// A canonical unit identity. Units remain opaque to the substance-agnostic engine.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UnitId(String);

impl UnitId {
    /// Parses a non-empty ASCII unit identity without leading or trailing whitespace.
    ///
    /// # Errors
    ///
    /// Returns [`ModelArtifactError::InvalidUnitIdentity`] for a non-canonical identity.
    pub fn parse(value: &str) -> Result<Self, ModelArtifactError> {
        if value.is_empty()
            || value.trim() != value
            || !value.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(ModelArtifactError::InvalidUnitIdentity {
                value: value.to_owned(),
            });
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for UnitId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One executable rule and its immutable scalar parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct RuleDefinition {
    compartment: CompartmentId,
    substance: SubstanceId,
    expression: RuleExpr,
    disposition: PartitionExpr,
    parameters: BTreeMap<ParameterId, u64>,
}

impl RuleDefinition {
    /// Constructs a rule after canonicalising parameters and rejecting repeated identities.
    ///
    /// # Errors
    ///
    /// Returns [`ModelArtifactError::DuplicateParameter`] or
    /// [`ModelArtifactError::NonFiniteParameter`] when parameters are not canonical.
    pub fn new(
        compartment: CompartmentId,
        substance: SubstanceId,
        expression: RuleExpr,
        disposition: PartitionExpr,
        parameters: impl IntoIterator<Item = (ParameterId, f64)>,
    ) -> Result<Self, ModelArtifactError> {
        let semantics = expression.numerical_semantics_version();
        let mut values = BTreeMap::new();
        for (parameter, value) in parameters {
            let normalized =
                semantics
                    .normalize(value)
                    .map_err(|_| ModelArtifactError::NonFiniteParameter {
                        compartment: compartment.clone(),
                        substance: substance.clone(),
                        parameter: parameter.clone(),
                        bits: value.to_bits(),
                    })?;
            if values
                .insert(parameter.clone(), normalized.to_bits())
                .is_some()
            {
                return Err(ModelArtifactError::DuplicateParameter {
                    compartment,
                    substance,
                    parameter,
                });
            }
        }
        Ok(Self {
            compartment,
            substance,
            expression,
            disposition,
            parameters: values,
        })
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
    pub fn expression(&self) -> &RuleExpr {
        &self.expression
    }
    #[must_use]
    pub fn disposition(&self) -> &PartitionExpr {
        &self.disposition
    }
    pub fn parameters(&self) -> impl ExactSizeIterator<Item = (&ParameterId, f64)> {
        self.parameters
            .iter()
            .map(|(id, bits)| (id, f64::from_bits(*bits)))
    }
}

/// A SHA-256 content address for canonical model-artifact bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModelDigest([u8; 32]);

impl ModelDigest {
    /// Constructs a digest value from its fixed-width representation.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }

    #[must_use]
    pub fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut result = String::with_capacity(64);
        for byte in self.0 {
            result.push(char::from(HEX[usize::from(byte >> 4)]));
            result.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        result
    }
}

impl Display for ModelDigest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

/// A complete immutable model value whose identity covers all execution inputs.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelArtifact {
    topology: Topology,
    registry: SubstanceRegistry,
    initial_stocks: InitialStocks,
    projections: ProjectionSet,
    calendar: FixedStepCalendar,
    horizon: RunHorizon,
    forcings: BTreeMap<ForcingId, ForcingSeries>,
    tables: BTreeMap<TableId, InterpolationTable>,
    rules: BTreeMap<(CompartmentId, SubstanceId), RuleDefinition>,
    units: BTreeMap<SubstanceId, UnitId>,
    versions: ModelVersions,
    canonical_bytes: Box<[u8]>,
    digest: ModelDigest,
}

impl ModelArtifact {
    /// Starts a builder with the structurally bound components.
    #[must_use]
    pub fn builder(
        topology: Topology,
        registry: SubstanceRegistry,
        initial_stocks: InitialStocks,
        calendar: FixedStepCalendar,
        horizon: RunHorizon,
    ) -> ModelArtifactBuilder {
        ModelArtifactBuilder {
            topology,
            registry,
            initial_stocks,
            calendar,
            horizon,
            projections: None,
            forcings: Vec::new(),
            tables: Vec::new(),
            rules: Vec::new(),
            units: Vec::new(),
            versions: ModelVersions::default(),
        }
    }

    #[must_use]
    pub fn digest(&self) -> ModelDigest {
        self.digest
    }
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    #[must_use]
    pub fn topology(&self) -> &Topology {
        &self.topology
    }
    #[must_use]
    pub fn registry(&self) -> &SubstanceRegistry {
        &self.registry
    }
    #[must_use]
    pub fn initial_stocks(&self) -> &InitialStocks {
        &self.initial_stocks
    }
    #[must_use]
    pub fn projections(&self) -> &ProjectionSet {
        &self.projections
    }
    #[must_use]
    pub fn calendar(&self) -> FixedStepCalendar {
        self.calendar
    }
    #[must_use]
    pub fn horizon(&self) -> RunHorizon {
        self.horizon
    }
    #[must_use]
    pub fn versions(&self) -> ModelVersions {
        self.versions
    }
    pub fn forcings(&self) -> impl ExactSizeIterator<Item = &ForcingSeries> {
        self.forcings.values()
    }
    pub fn tables(&self) -> impl ExactSizeIterator<Item = &InterpolationTable> {
        self.tables.values()
    }
    pub fn rules(&self) -> impl ExactSizeIterator<Item = &RuleDefinition> {
        self.rules.values()
    }
    pub fn units(&self) -> impl ExactSizeIterator<Item = (&SubstanceId, &UnitId)> {
        self.units.iter()
    }
}

/// Builder for a model artifact's optional collections and version selection.
pub struct ModelArtifactBuilder {
    topology: Topology,
    registry: SubstanceRegistry,
    initial_stocks: InitialStocks,
    calendar: FixedStepCalendar,
    horizon: RunHorizon,
    projections: Option<ProjectionSet>,
    forcings: Vec<ForcingSeries>,
    tables: Vec<InterpolationTable>,
    rules: Vec<RuleDefinition>,
    units: Vec<(SubstanceId, UnitId)>,
    versions: ModelVersions,
}

impl ModelArtifactBuilder {
    #[must_use]
    pub fn with_projections(mut self, projections: ProjectionSet) -> Self {
        self.projections = Some(projections);
        self
    }
    #[must_use]
    pub fn with_forcings(mut self, forcings: Vec<ForcingSeries>) -> Self {
        self.forcings = forcings;
        self
    }
    #[must_use]
    pub fn with_tables(mut self, tables: Vec<InterpolationTable>) -> Self {
        self.tables = tables;
        self
    }
    #[must_use]
    pub fn with_rules(mut self, rules: Vec<RuleDefinition>) -> Self {
        self.rules = rules;
        self
    }
    #[must_use]
    pub fn with_units(mut self, units: Vec<(SubstanceId, UnitId)>) -> Self {
        self.units = units;
        self
    }
    #[must_use]
    pub fn with_versions(mut self, versions: ModelVersions) -> Self {
        self.versions = versions;
        self
    }

    /// Validates cross-component bindings, canonicalises the complete value, and hashes it.
    ///
    /// # Errors
    ///
    /// Returns [`ModelArtifactError`] for a component mismatch or canonical encoding failure.
    pub fn build(self) -> Result<ModelArtifact, ModelArtifactError> {
        if self.initial_stocks.topology() != &self.topology {
            return Err(ModelArtifactError::InitialStocksTopologyMismatch);
        }
        if self.initial_stocks.registry() != &self.registry {
            return Err(ModelArtifactError::InitialStocksRegistryMismatch);
        }
        let projections = self
            .projections
            .ok_or(ModelArtifactError::MissingProjectionSet)?;
        for specification in projections.iter() {
            if specification.rule_ir_version() != self.versions.rule_ir {
                return Err(ModelArtifactError::ProjectionRuleIrVersionMismatch {
                    projection: specification.id().clone(),
                });
            }
            if specification.numerical_semantics_version() != self.versions.numerical_semantics {
                return Err(ModelArtifactError::ProjectionNumericalVersionMismatch {
                    projection: specification.id().clone(),
                });
            }
        }
        let mut forcings = BTreeMap::new();
        for forcing in self.forcings {
            if forcing.horizon() != self.horizon {
                return Err(ModelArtifactError::ForcingHorizonMismatch {
                    forcing: forcing.id().clone(),
                });
            }
            let id = forcing.id().clone();
            if forcings.insert(id.clone(), forcing).is_some() {
                return Err(ModelArtifactError::DuplicateForcing { forcing: id });
            }
        }
        let mut tables = BTreeMap::new();
        for table in self.tables {
            let id = table.id().clone();
            if table.numerical_semantics_version() != self.versions.numerical_semantics {
                return Err(ModelArtifactError::TableNumericalVersionMismatch { table: id });
            }
            if tables.insert(id.clone(), table).is_some() {
                return Err(ModelArtifactError::DuplicateTable { table: id });
            }
        }
        let mut rules = BTreeMap::new();
        for rule in self.rules {
            let key = (rule.compartment.clone(), rule.substance.clone());
            match self.topology.endpoint(&key.0) {
                None => {
                    return Err(ModelArtifactError::UnknownRuleCompartment { compartment: key.0 });
                }
                Some(TopologyEndpoint::Boundary(_)) => {
                    return Err(ModelArtifactError::BoundaryAccountRule { account: key.0 });
                }
                Some(TopologyEndpoint::Finite(_)) => {}
            }
            if !self.registry.contains(&key.1) {
                return Err(ModelArtifactError::UnknownRuleSubstance { substance: key.1 });
            }
            if rule.expression.rule_ir_version() != self.versions.rule_ir
                || rule.disposition.rule_ir_version() != self.versions.rule_ir
            {
                return Err(ModelArtifactError::RuleIrVersionMismatch {
                    compartment: key.0,
                    substance: key.1,
                });
            }
            if rule.expression.numerical_semantics_version() != self.versions.numerical_semantics
                || rule.disposition.numerical_semantics_version()
                    != self.versions.numerical_semantics
            {
                return Err(ModelArtifactError::RuleNumericalVersionMismatch {
                    compartment: key.0,
                    substance: key.1,
                });
            }
            validate_rule_references(&rule, &forcings, &tables, &projections)?;
            if rules.insert(key.clone(), rule).is_some() {
                return Err(ModelArtifactError::DuplicateRule {
                    compartment: key.0,
                    substance: key.1,
                });
            }
        }
        let mut units = BTreeMap::new();
        for (substance, unit) in self.units {
            if !self.registry.contains(&substance) {
                return Err(ModelArtifactError::UnknownUnitSubstance { substance });
            }
            if units.insert(substance.clone(), unit).is_some() {
                return Err(ModelArtifactError::DuplicateUnit { substance });
            }
        }
        for substance in self.registry.iter() {
            if !units.contains_key(substance) {
                return Err(ModelArtifactError::MissingUnit {
                    substance: substance.clone(),
                });
            }
        }
        let mut artifact = ModelArtifact {
            topology: self.topology,
            registry: self.registry,
            initial_stocks: self.initial_stocks,
            projections,
            calendar: self.calendar,
            horizon: self.horizon,
            forcings,
            tables,
            rules,
            units,
            versions: self.versions,
            canonical_bytes: Box::new([]),
            digest: ModelDigest([0; 32]),
        };
        let bytes = artifact.versions.canonical_encoding.encode(&artifact)?;
        let mut hasher = Sha256::new();
        hasher.update(b"incidence:model-artifact:v1\0");
        hasher.update(&bytes);
        let hash: [u8; 32] = hasher.finalize().into();
        artifact.canonical_bytes = bytes.into_boxed_slice();
        artifact.digest = ModelDigest(hash);
        Ok(artifact)
    }
}

/// Reports why a complete model artifact could not be constructed.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ModelArtifactError {
    /// Fires when an initial-stock value is bound to a different topology snapshot.
    #[error("initial stocks are bound to a different topology")]
    InitialStocksTopologyMismatch,
    /// Fires when an initial-stock value is bound to a different registry snapshot.
    #[error("initial stocks are bound to a different substance registry")]
    InitialStocksRegistryMismatch,
    /// Fires when no projection set (including an explicitly empty set) was supplied.
    #[error("model artifact requires an explicit projection set")]
    MissingProjectionSet,
    /// Fires when a projection's rule IR version differs from the artifact selection.
    #[error("projection `{projection}` has a different rule IR version")]
    ProjectionRuleIrVersionMismatch { projection: ProjectionId },
    /// Fires when a projection's numerical version differs from the artifact selection.
    #[error("projection `{projection}` has a different numerical semantics version")]
    ProjectionNumericalVersionMismatch { projection: ProjectionId },
    /// Fires when a forcing does not cover the artifact horizon exactly.
    #[error("forcing `{forcing}` does not cover the model horizon")]
    ForcingHorizonMismatch { forcing: ForcingId },
    /// Fires when a forcing identity is repeated.
    #[error("duplicate forcing `{forcing}`")]
    DuplicateForcing { forcing: ForcingId },
    /// Fires when an interpolation-table identity is repeated.
    #[error("duplicate interpolation table `{table}`")]
    DuplicateTable { table: TableId },
    /// Fires when a table's numerical semantics version differs from the artifact selection.
    #[error("interpolation table `{table}` has a different numerical semantics version")]
    TableNumericalVersionMismatch { table: TableId },
    /// Fires when a rule names an endpoint absent from the topology.
    #[error("rule compartment `{compartment}` is absent from the topology")]
    UnknownRuleCompartment { compartment: CompartmentId },
    /// Fires when a rule is assigned to a boundary account rather than a finite compartment.
    #[error("boundary account `{account}` cannot own a rule")]
    BoundaryAccountRule { account: CompartmentId },
    /// Fires when a rule names a substance absent from the registry.
    #[error("rule substance `{substance}` is absent from the registry")]
    UnknownRuleSubstance { substance: SubstanceId },
    /// Fires when two rules name the same compartment-substance coordinate.
    #[error("duplicate rule for compartment `{compartment}`, substance `{substance}`")]
    DuplicateRule {
        compartment: CompartmentId,
        substance: SubstanceId,
    },
    /// Fires when a rule's IR version differs from the artifact selection.
    #[error(
        "rule for compartment `{compartment}`, substance `{substance}` has a different rule IR version"
    )]
    RuleIrVersionMismatch {
        compartment: CompartmentId,
        substance: SubstanceId,
    },
    /// Fires when a rule's numerical version differs from the artifact selection.
    #[error(
        "rule for compartment `{compartment}`, substance `{substance}` has a different numerical semantics version"
    )]
    RuleNumericalVersionMismatch {
        compartment: CompartmentId,
        substance: SubstanceId,
    },
    /// Fires when one rule parameter identity occurs twice.
    #[error(
        "duplicate parameter `{parameter}` for compartment `{compartment}`, substance `{substance}`"
    )]
    DuplicateParameter {
        compartment: CompartmentId,
        substance: SubstanceId,
        parameter: ParameterId,
    },
    /// Fires when a rule parameter is NaN or infinite.
    #[error(
        "non-finite parameter `{parameter}` for compartment `{compartment}`, substance `{substance}`: bits {bits:#018x}"
    )]
    NonFiniteParameter {
        compartment: CompartmentId,
        substance: SubstanceId,
        parameter: ParameterId,
        bits: u64,
    },
    /// Fires when an expression refers to an undeclared rule parameter.
    #[error(
        "rule for compartment `{compartment}`, substance `{substance}` refers to missing parameter `{parameter}`"
    )]
    MissingRuleParameter {
        compartment: CompartmentId,
        substance: SubstanceId,
        parameter: ParameterId,
    },
    /// Fires when an expression or partition refers to an absent forcing series.
    #[error(
        "rule for compartment `{compartment}`, substance `{substance}` refers to missing forcing `{forcing}`"
    )]
    MissingRuleForcing {
        compartment: CompartmentId,
        substance: SubstanceId,
        forcing: ForcingId,
    },
    /// Fires when an expression refers to an absent interpolation table.
    #[error(
        "rule for compartment `{compartment}`, substance `{substance}` refers to missing table `{table}`"
    )]
    MissingRuleTable {
        compartment: CompartmentId,
        substance: SubstanceId,
        table: TableId,
    },
    /// Fires when an expression refers to an absent projection specification.
    #[error(
        "rule for compartment `{compartment}`, substance `{substance}` refers to missing projection `{projection}`"
    )]
    MissingRuleProjection {
        compartment: CompartmentId,
        substance: SubstanceId,
        projection: ProjectionId,
    },
    /// Fires when a unit identity is empty, non-ASCII, or padded with whitespace.
    #[error("invalid canonical unit identity `{value}`")]
    InvalidUnitIdentity { value: String },
    /// Fires when a unit names an unmodelled substance.
    #[error("unit declaration names unmodelled substance `{substance}`")]
    UnknownUnitSubstance { substance: SubstanceId },
    /// Fires when a substance unit is declared more than once.
    #[error("duplicate unit declaration for substance `{substance}`")]
    DuplicateUnit { substance: SubstanceId },
    /// Fires when a modelled substance has no unit declaration.
    #[error("missing unit declaration for substance `{substance}`")]
    MissingUnit { substance: SubstanceId },
    /// Fires when canonical bytes cannot represent a field.
    #[error(transparent)]
    CanonicalEncoding(#[from] CanonicalEncodingError),
}

fn validate_rule_references(
    rule: &RuleDefinition,
    forcings: &BTreeMap<ForcingId, ForcingSeries>,
    tables: &BTreeMap<TableId, InterpolationTable>,
    projections: &ProjectionSet,
) -> Result<(), ModelArtifactError> {
    let mut parameters = BTreeSet::new();
    let mut forcing_refs = BTreeSet::new();
    let mut table_refs = BTreeSet::new();
    let mut projection_refs = BTreeSet::new();
    collect_expression_references(
        &rule.expression,
        &mut parameters,
        &mut forcing_refs,
        &mut table_refs,
        &mut projection_refs,
    );
    if let PartitionExprView::ExogenousSeries { series, .. } = rule.disposition.view() {
        forcing_refs.insert(series.id().clone());
    }
    for parameter in parameters {
        if !rule.parameters.contains_key(&parameter) {
            return Err(ModelArtifactError::MissingRuleParameter {
                compartment: rule.compartment.clone(),
                substance: rule.substance.clone(),
                parameter,
            });
        }
    }
    for forcing in forcing_refs {
        if !forcings.contains_key(&forcing) {
            return Err(ModelArtifactError::MissingRuleForcing {
                compartment: rule.compartment.clone(),
                substance: rule.substance.clone(),
                forcing,
            });
        }
    }
    for table in table_refs {
        if !tables.contains_key(&table) {
            return Err(ModelArtifactError::MissingRuleTable {
                compartment: rule.compartment.clone(),
                substance: rule.substance.clone(),
                table,
            });
        }
    }
    let declared_projections = projections
        .iter()
        .map(|specification| specification.id())
        .collect::<BTreeSet<_>>();
    for projection in projection_refs {
        if !declared_projections.contains(&projection) {
            return Err(ModelArtifactError::MissingRuleProjection {
                compartment: rule.compartment.clone(),
                substance: rule.substance.clone(),
                projection,
            });
        }
    }
    Ok(())
}

fn collect_expression_references(
    expression: &RuleExpr,
    parameters: &mut BTreeSet<ParameterId>,
    forcings: &mut BTreeSet<ForcingId>,
    tables: &mut BTreeSet<TableId>,
    projections: &mut BTreeSet<ProjectionId>,
) {
    match expression.view() {
        RuleExprView::Parameter(reference) => {
            parameters.insert(reference.id().clone());
        }
        RuleExprView::Forcing(reference) => {
            forcings.insert(reference.id().clone());
        }
        RuleExprView::Projection(reference) => {
            projections.insert(reference.id().clone());
        }
        RuleExprView::InterpolatedTable { table, input } => {
            tables.insert(table.id().clone());
            collect_expression_references(input, parameters, forcings, tables, projections);
        }
        RuleExprView::Add { lhs, rhs }
        | RuleExprView::Subtract { lhs, rhs }
        | RuleExprView::Multiply { lhs, rhs }
        | RuleExprView::Divide { lhs, rhs }
        | RuleExprView::Minimum { lhs, rhs }
        | RuleExprView::Maximum { lhs, rhs }
        | RuleExprView::Comparison { lhs, rhs, .. } => {
            collect_expression_references(lhs, parameters, forcings, tables, projections);
            collect_expression_references(rhs, parameters, forcings, tables, projections);
        }
        RuleExprView::Clamp {
            value,
            lower,
            upper,
        } => {
            collect_expression_references(value, parameters, forcings, tables, projections);
            collect_expression_references(lower, parameters, forcings, tables, projections);
            collect_expression_references(upper, parameters, forcings, tables, projections);
        }
        RuleExprView::Select {
            condition,
            when_true,
            when_false,
        } => {
            collect_expression_references(condition, parameters, forcings, tables, projections);
            collect_expression_references(when_true, parameters, forcings, tables, projections);
            collect_expression_references(when_false, parameters, forcings, tables, projections);
        }
        RuleExprView::Input(_) | RuleExprView::Literal(_) => {}
    }
}

impl CanonicalEncode for ModelArtifact {
    fn root_tag(&self) -> u16 {
        0x0020
    }

    fn encode_payload(
        &self,
        writer: &mut CanonicalPayloadWriter,
    ) -> Result<(), CanonicalEncodingError> {
        self.versions.rule_ir.encode_payload(writer)?;
        self.versions.interpreter.encode_payload(writer)?;
        self.versions.numerical_semantics.encode_payload(writer)?;
        self.versions.canonical_encoding.encode_payload(writer)?;
        self.topology.encode_payload(writer)?;
        self.registry.encode_payload(writer)?;
        self.initial_stocks.encode_payload(writer)?;
        self.calendar.encode_payload(writer)?;
        self.horizon.encode_payload(writer)?;
        writer.write_count(
            CanonicalField::ArtifactProjections,
            self.projections.iter().len(),
        )?;
        for specification in self.projections.iter() {
            specification.encode_payload(writer)?;
        }
        writer.write_count(
            CanonicalField::ArtifactProjectorStates,
            self.projections.initial_states().len(),
        )?;
        for (_, state) in self.projections.initial_states() {
            state.encode_payload(writer)?;
        }
        writer.write_count(CanonicalField::ArtifactForcings, self.forcings.len())?;
        for forcing in self.forcings.values() {
            forcing.encode_payload(writer)?;
        }
        writer.write_count(CanonicalField::ArtifactTables, self.tables.len())?;
        for table in self.tables.values() {
            table.encode_payload(writer)?;
        }
        writer.write_count(CanonicalField::ArtifactRules, self.rules.len())?;
        for rule in self.rules.values() {
            writer.write_string(
                CanonicalField::ArtifactRuleCompartment,
                rule.compartment.as_str(),
            )?;
            writer.write_string(
                CanonicalField::ArtifactRuleSubstance,
                rule.substance.as_str(),
            )?;
            rule.expression.encode_payload(writer)?;
            rule.disposition.encode_payload(writer)?;
            writer.write_count(CanonicalField::ArtifactParameters, rule.parameters.len())?;
            for (parameter, bits) in &rule.parameters {
                writer.write_string(
                    CanonicalField::ArtifactParameterIdentity,
                    parameter.as_str(),
                )?;
                writer.write_scalar(
                    CanonicalField::ArtifactParameterValue,
                    f64::from_bits(*bits),
                )?;
            }
        }
        writer.write_count(CanonicalField::ArtifactUnits, self.units.len())?;
        for (substance, unit) in &self.units {
            writer.write_string(CanonicalField::ArtifactUnitSubstance, substance.as_str())?;
            writer.write_string(CanonicalField::ArtifactUnitIdentity, unit.as_str())?;
        }
        Ok(())
    }
}

/// An append-only in-memory content-addressed repository.
#[derive(Default)]
pub struct ModelArtifactArchive {
    artifacts: BTreeMap<ModelDigest, Arc<ModelArtifact>>,
}

impl ModelArtifactArchive {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Retains an artifact under its derived digest and returns its content address.
    ///
    /// # Errors
    ///
    /// Returns [`ModelArtifactArchiveError::DigestCollision`] rather than guessing if a digest
    /// already identifies different canonical bytes.
    pub fn insert(
        &mut self,
        artifact: ModelArtifact,
    ) -> Result<ModelDigest, ModelArtifactArchiveError> {
        let digest = artifact.digest();
        if let Some(existing) = self.artifacts.get(&digest) {
            if existing.canonical_bytes() != artifact.canonical_bytes() {
                return Err(ModelArtifactArchiveError::DigestCollision { digest });
            }
            return Ok(digest);
        }
        self.artifacts.insert(digest, Arc::new(artifact));
        Ok(digest)
    }

    /// Retrieves any retained historical artifact without granting mutation authority.
    #[must_use]
    pub fn get(&self, digest: &ModelDigest) -> Option<Arc<ModelArtifact>> {
        self.artifacts.get(digest).cloned()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.artifacts.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

/// Reports a content-address collision in an artifact archive.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum ModelArtifactArchiveError {
    /// Fires if unequal canonical artifacts produce the same digest.
    #[error("model digest collision at `{digest}`")]
    DigestCollision { digest: ModelDigest },
}

/// Backwards-compatible concise name for the in-memory archive.
pub type ModelArtifactStore = ModelArtifactArchive;

#![allow(clippy::expect_used)]
use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::execution_bindings::{ExecutionBindings, TransferBranchBinding};
use incidence_core::forcing::ForcingSeries;
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::InitialStocks;
use incidence_core::model_artifact::{ModelArtifact, RuleDefinition, UnitId};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::partition_expression::PartitionExpr;
use incidence_core::projection::ProjectionSet;
use incidence_core::rule_expression::RuleExpr;
use incidence_core::rule_reference::{ForcingId, ForcingRef, TransferBranchId};
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TimestepDuration, TimestepIndex,
};
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};
use incidence_core::versions::RuleIrVersion;

pub fn artifact() -> ModelArtifact {
    let source = CompartmentId::parse("source").expect("id");
    let outside = CompartmentId::parse("outside").expect("id");
    let topology = Topology::new(
        [
            TopologyEndpoint::Finite(FiniteCompartment::new(source.clone())),
            TopologyEndpoint::Boundary(BoundaryAccount::new(outside.clone())),
        ],
        [DirectedConnection::new(source.clone(), outside.clone())],
    )
    .expect("topology");
    let water = SubstanceId::parse("water").expect("id");
    let registry = SubstanceRegistry::new([water.clone()]).expect("registry");
    let stocks = InitialStocks::new(
        &topology,
        &registry,
        [(
            source.clone(),
            SparseSubstanceVector::new(
                &registry,
                [(
                    water.clone(),
                    NonNegativeAmount::try_from(10.0).expect("amount"),
                )],
            )
            .expect("vector"),
        )],
    )
    .expect("stocks");
    let horizon = RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(1)).expect("horizon");
    let calendar = FixedStepCalendar::new(
        CalendarOrigin::new(CalendarInstant::from_unix_seconds(0)),
        TimestepDuration::from_seconds(1).expect("duration"),
    );
    let forcing_id = ForcingId::parse("release").expect("id");
    let branch = TransferBranchId::parse("out").expect("id");
    let rule = RuleDefinition::new(
        source.clone(),
        water.clone(),
        RuleExpr::forcing(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            ForcingRef::new(forcing_id.clone()),
        ),
        PartitionExpr::release_all(
            RuleIrVersion::V1,
            NumericalSemanticsVersion::V1,
            branch.clone(),
        ),
        [],
    )
    .expect("rule");
    let bindings = ExecutionBindings::new(
        [TransferBranchBinding::new(
            source,
            water.clone(),
            branch,
            outside,
        )],
        [],
    )
    .expect("bindings");
    ModelArtifact::builder(topology, registry, stocks, calendar, horizon)
        .with_projections(ProjectionSet::new(vec![], vec![]).expect("projections"))
        .with_forcings(vec![
            ForcingSeries::new(forcing_id, horizon, vec![1.0, 2.0]).expect("forcing"),
        ])
        .with_rules(vec![rule])
        .with_execution_bindings(bindings)
        .with_units(vec![(water, UnitId::parse("m3").expect("unit"))])
        .build()
        .expect("artifact")
}

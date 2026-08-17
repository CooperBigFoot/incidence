//! Canonical root tags uniquely identify public root value types.

use std::collections::BTreeSet;

use incidence_core::canonical_encoding::CanonicalEncode;
use incidence_core::forcing::ForcingSeries;
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::interpolation_table::{InterpolationBoundaryPolicy, InterpolationTable};
use incidence_core::numerical_semantics::NumericalSemanticsVersion;
use incidence_core::projection::{
    AuthoritativeFactSelector, BoundedLagSpec, InitialProjectionValue, InitialProjectorState,
    ProjectionSource, ProjectionSpec,
};
use incidence_core::rule_reference::{ForcingId, ProjectionId, TableId};
use incidence_core::temporal::{RunHorizon, TimestepIndex};
use incidence_core::versions::RuleIrVersion;

#[test]
fn canonical_root_tags_are_unique_across_projection_and_forcing_types() {
    let selector = AuthoritativeFactSelector::IncomingTransferAmount {
        compartment: CompartmentId::parse("reservoir").unwrap(),
        substance: SubstanceId::parse("water").unwrap(),
    };
    let projection: ProjectionSpec = BoundedLagSpec::new(
        RuleIrVersion::V1,
        NumericalSemanticsVersion::V1,
        ProjectionId::parse("inflow-lag").unwrap(),
        ProjectionSource::AuthoritativeFact(selector),
        1,
    )
    .unwrap()
    .into();
    let initial_state = InitialProjectorState::new(
        ProjectionId::parse("inflow-lag").unwrap(),
        NumericalSemanticsVersion::V1,
        vec![InitialProjectionValue::Extensive(0.0)],
    )
    .unwrap();
    let horizon = RunHorizon::new(TimestepIndex::new(0), TimestepIndex::new(0)).unwrap();
    let forcing =
        ForcingSeries::new(ForcingId::parse("rainfall").unwrap(), horizon, vec![0.0]).unwrap();
    let table = InterpolationTable::new(
        TableId::parse("rating-curve").unwrap(),
        NumericalSemanticsVersion::V1,
        InterpolationBoundaryPolicy::ClampToEndpoint,
        vec![0.0, 1.0],
        vec![0.0, 1.0],
    )
    .unwrap();

    let tags = [
        projection.root_tag(),
        initial_state.root_tag(),
        forcing.root_tag(),
        table.root_tag(),
    ];
    assert_eq!(tags.into_iter().collect::<BTreeSet<_>>().len(), tags.len());
}

use incidence_core::endpoints::{BoundaryAccount, FiniteCompartment};
use incidence_core::identity::{CompartmentId, SubstanceId};
use incidence_core::initial_stocks::{InitialStocks, InitialStocksError};
use incidence_core::non_negative_amount::NonNegativeAmount;
use incidence_core::presence::ValueState;
use incidence_core::sparse_substance_vector::SparseSubstanceVector;
use incidence_core::substance_registry::SubstanceRegistry;
use incidence_core::topology::{DirectedConnection, Topology, TopologyEndpoint};

fn id(value: &str) -> CompartmentId {
    match CompartmentId::parse(value) {
        Ok(identity) => identity,
        Err(error) => panic!("invalid compartment fixture `{value}`: {error}"),
    }
}

fn substance(value: &str) -> SubstanceId {
    match SubstanceId::parse(value) {
        Ok(identity) => identity,
        Err(error) => panic!("invalid substance fixture `{value}`: {error}"),
    }
}

fn amount(value: f64) -> NonNegativeAmount {
    match NonNegativeAmount::try_from(value) {
        Ok(amount) => amount,
        Err(error) => panic!("invalid amount fixture `{value}`: {error}"),
    }
}

fn registry(values: &[&str]) -> SubstanceRegistry {
    match SubstanceRegistry::new(values.iter().map(|value| substance(value))) {
        Ok(registry) => registry,
        Err(error) => panic!("invalid registry fixture: {error}"),
    }
}

fn vector(registry: &SubstanceRegistry, entries: &[(&str, f64)]) -> SparseSubstanceVector {
    match SparseSubstanceVector::new(
        registry,
        entries
            .iter()
            .map(|(identity, value)| (substance(identity), amount(*value))),
    ) {
        Ok(vector) => vector,
        Err(error) => panic!("invalid vector fixture: {error}"),
    }
}

fn topology(endpoints: Vec<TopologyEndpoint>, connections: Vec<DirectedConnection>) -> Topology {
    match Topology::new(endpoints, connections) {
        Ok(topology) => topology,
        Err(error) => panic!("invalid topology fixture: {error}"),
    }
}

fn finite(value: &str) -> TopologyEndpoint {
    TopologyEndpoint::Finite(FiniteCompartment::new(id(value)))
}

fn boundary(value: &str) -> TopologyEndpoint {
    TopologyEndpoint::Boundary(BoundaryAccount::new(id(value)))
}

#[test]
fn valid_binding_accessors_topology_order_and_repeatability() {
    let topology = topology(
        vec![
            boundary("catchment"),
            finite("upstream"),
            finite("downstream"),
            boundary("sea"),
        ],
        vec![
            DirectedConnection::new(id("catchment"), id("upstream")),
            DirectedConnection::new(id("upstream"), id("downstream")),
            DirectedConnection::new(id("downstream"), id("sea")),
        ],
    );
    let registry = registry(&["water", "salt"]);
    let downstream = vector(&registry, &[("salt", 4.25)]);
    let upstream = vector(&registry, &[("water", 2.0)]);

    let first = InitialStocks::new(
        &topology,
        &registry,
        [
            (id("downstream"), downstream.clone()),
            (id("upstream"), upstream.clone()),
        ],
    )
    .expect("valid initial stocks must bind");
    let second = InitialStocks::new(
        &topology,
        &registry,
        [(id("upstream"), upstream), (id("downstream"), downstream)],
    )
    .expect("valid initial stocks must bind");

    assert_eq!(first, second);
    assert_eq!(first.topology(), &topology);
    assert_eq!(second.topology(), &topology);
    assert_eq!(first.registry(), &registry);
    assert_eq!(second.registry(), &registry);

    let expected = vec![
        (
            "upstream".to_owned(),
            vec![("water".to_owned(), 0x4000000000000000)],
        ),
        (
            "downstream".to_owned(),
            vec![("salt".to_owned(), 0x4011000000000000)],
        ),
    ];
    let collect = |stocks: &InitialStocks| {
        stocks
            .iter()
            .map(|(compartment, vector)| {
                (
                    compartment.as_str().to_owned(),
                    vector
                        .iter()
                        .map(|(substance, amount)| {
                            (substance.as_str().to_owned(), amount.value().to_bits())
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(collect(&first), expected);
    assert_eq!(collect(&second), expected);
}

#[test]
fn omitted_pairs_are_explicit_modelled_zero() {
    let topology = topology(vec![finite("upstream"), finite("downstream")], vec![]);
    let registry = registry(&["water", "salt"]);
    let stocks = InitialStocks::new(
        &topology,
        &registry,
        [(id("upstream"), vector(&registry, &[("water", 2.0)]))],
    )
    .expect("valid initial stocks must bind");
    let water = substance("water");
    let salt = substance("salt");
    let nitrate = substance("nitrate");

    let present = stocks
        .amount(&id("upstream"), &water)
        .expect("known finite compartment lookup must succeed");
    assert_eq!(present, ValueState::Present(amount(2.0)));
    let ValueState::Present(payload) = present else {
        panic!("registered supplied amount must be present");
    };
    assert_eq!(payload.value().to_bits(), 0x4000000000000000);

    for result in [
        stocks.amount(&id("upstream"), &salt),
        stocks.amount(&id("downstream"), &water),
        stocks.amount(&id("downstream"), &salt),
    ] {
        assert_eq!(result, Ok(ValueState::Present(NonNegativeAmount::ZERO)));
        let Ok(ValueState::Present(payload)) = result else {
            panic!("omitted registered amount must be present zero");
        };
        assert_eq!(payload.value().to_bits(), 0x0000000000000000);
    }

    assert_eq!(
        stocks.amount(&id("upstream"), &nitrate),
        Ok(ValueState::NotModelled)
    );
    assert_eq!(
        stocks.amount(&id("downstream"), &nitrate),
        Ok(ValueState::NotModelled)
    );

    let traversed: Vec<String> = stocks
        .iter()
        .map(|(compartment, _)| compartment.as_str().to_owned())
        .collect();
    assert_eq!(traversed, vec!["upstream".to_owned()]);
}

#[test]
fn unknown_assignment_fails() {
    let topology = topology(vec![finite("upstream")], vec![]);
    let registry = registry(&["water"]);
    let result = InitialStocks::new(
        &topology,
        &registry,
        [(id("unknown"), vector(&registry, &[]))],
    );

    assert_eq!(
        result,
        Err(InitialStocksError::UnknownCompartment {
            compartment: id("unknown"),
        })
    );
    assert_eq!(
        result
            .expect_err("unknown assignment must fail")
            .to_string(),
        "initial stock compartment `unknown` is not declared in the topology"
    );
}

#[test]
fn boundary_account_assignment_fails() {
    let topology = topology(vec![boundary("catchment")], vec![]);
    let registry = registry(&["water"]);
    let result = InitialStocks::new(
        &topology,
        &registry,
        [(id("catchment"), vector(&registry, &[]))],
    );

    assert_eq!(
        result,
        Err(InitialStocksError::BoundaryAccountStock {
            account: id("catchment"),
        })
    );
    assert_eq!(
        result
            .expect_err("boundary-account assignment must fail")
            .to_string(),
        "boundary account `catchment` cannot hold initial stock"
    );
}

#[test]
fn registry_mismatch_fails_structurally() {
    let topology = topology(vec![finite("upstream")], vec![]);
    let different_registry = registry(&["water", "nitrate"]);
    let registry = registry(&["water", "salt"]);
    let result = InitialStocks::new(
        &topology,
        &registry,
        [(id("upstream"), vector(&different_registry, &[]))],
    );

    assert_eq!(
        result,
        Err(InitialStocksError::RegistryMismatch {
            compartment: id("upstream"),
        })
    );
    assert_eq!(
        result.expect_err("registry mismatch must fail").to_string(),
        "initial stock for compartment `upstream` is bound to a different substance registry"
    );
}

#[test]
fn duplicate_compartment_fails_before_map_overwrite() {
    let topology = topology(vec![finite("upstream")], vec![]);
    let registry = registry(&["water"]);
    let result = InitialStocks::new(
        &topology,
        &registry,
        [
            (id("upstream"), vector(&registry, &[])),
            (id("upstream"), vector(&registry, &[("water", 2.0)])),
        ],
    );

    assert_eq!(
        result,
        Err(InitialStocksError::DuplicateCompartment {
            compartment: id("upstream"),
        })
    );
    assert_eq!(
        result
            .expect_err("duplicate compartment must fail")
            .to_string(),
        "duplicate initial stock entry for compartment `upstream`"
    );
}

#[test]
fn invalid_lookup_fails_loudly() {
    let topology = topology(vec![finite("upstream"), boundary("catchment")], vec![]);
    let registry = registry(&["water"]);
    let stocks = InitialStocks::new(&topology, &registry, [])
        .expect("empty initial stocks must bind to a non-empty model");
    let water = substance("water");

    let unknown = stocks.amount(&id("unknown"), &water);
    assert_eq!(
        unknown,
        Err(InitialStocksError::UnknownCompartment {
            compartment: id("unknown"),
        })
    );
    assert_eq!(
        unknown.expect_err("unknown lookup must fail").to_string(),
        "initial stock compartment `unknown` is not declared in the topology"
    );

    let boundary = stocks.amount(&id("catchment"), &water);
    assert_eq!(
        boundary,
        Err(InitialStocksError::BoundaryAccountStock {
            account: id("catchment"),
        })
    );
    assert_eq!(
        boundary.expect_err("boundary lookup must fail").to_string(),
        "boundary account `catchment` cannot hold initial stock"
    );
}

#[test]
fn empty_inputs_remain_valid_and_exact() {
    let topology = topology(vec![], vec![]);
    let registry = registry(&[]);
    let stocks = InitialStocks::new(&topology, &registry, [])
        .expect("empty initial stocks must bind to an empty model");

    assert!(stocks.topology().is_empty());
    assert!(stocks.registry().is_empty());
    assert_eq!(stocks.iter().next(), None);
}

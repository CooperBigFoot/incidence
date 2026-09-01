from __future__ import annotations

import hashlib
import json
from pathlib import Path

import incidence


RULE_COMPARTMENTS = (
    "muskingum",
    "lag",
    "linear-reservoir",
    "evaporation",
    "seepage",
    "reservoir-policy",
    "demand",
)
RUN_ID = bytes([0x71]) * 16
RUST_LOG_DIGEST = "52b6ae08c513f20a611ff271e86874fd5bfe47c12535c941a6fa1db4f56387e7"


def _expression_node(kind: str, lhs: dict, rhs: dict) -> dict:
    """Author the two V1 binary nodes not exposed as public binding combinators."""
    return {
        "rule_ir_version": "v1",
        "numerical_semantics_version": "v1",
        "expression": {
            "kind": kind,
            "lhs": lhs["expression"],
            "rhs": rhs["expression"],
        },
    }


def _fact(direction: str, compartment: str) -> dict:
    return {
        "kind": "authoritative_fact",
        "selector": {
            "kind": f"{direction}_transfer_amount",
            "compartment": compartment,
            "substance": "water",
        },
    }


def _reference(identifier: str, value_kind: str = "scalar") -> dict:
    return {"id": identifier, "value_kind": value_kind}


def _bounded_lag(identifier: str, direction: str, compartment: str, steps: int) -> dict:
    return {
        "rule_ir_version": "v1",
        "numerical_semantics_version": "v1",
        "id": identifier,
        "value_kind": "extensive",
        "spec": {
            "kind": "bounded_lag",
            "source": _fact(direction, compartment),
            "steps": steps,
        },
    }


def _finite_recurrence(
    identifier: str,
    state_kinds: list[str],
    inputs: list[dict],
    parameters: list[str],
    updates: list[dict],
    output_index: int,
) -> dict:
    return {
        "rule_ir_version": "v1",
        "numerical_semantics_version": "v1",
        "id": identifier,
        "value_kind": state_kinds[output_index],
        "spec": {
            "kind": "finite_recurrence",
            "state_kinds": state_kinds,
            "inputs": inputs,
            "parameters": [_reference(item) for item in parameters],
            "updates": updates,
            "output_index": output_index,
        },
    }


def _projection_document() -> dict:
    muskingum_update = incidence.add(
        incidence.add(
            incidence.mul(
                incidence.param("muskingum-c0"),
                incidence.input("muskingum-current-in"),
            ),
            incidence.mul(
                incidence.param("muskingum-c1"),
                incidence.projection("muskingum-in-lag"),
            ),
        ),
        incidence.mul(
            incidence.param("muskingum-c2"),
            incidence.projection("muskingum-out-lag"),
        ),
    )
    linear_previous = incidence.input("linear-previous-storage")
    linear_coefficient = incidence.param("linear-coefficient")
    linear_storage = incidence.mul(
        _expression_node(
            "subtract", incidence.literal(1.0), linear_coefficient
        ),
        linear_previous,
    )
    linear_release = incidence.mul(linear_coefficient, linear_previous)

    return {
        "specifications": [
            _bounded_lag("muskingum-in-lag", "incoming", "muskingum", 1),
            _bounded_lag("muskingum-out-lag", "outgoing", "muskingum", 1),
            _finite_recurrence(
                "muskingum-routed",
                ["extensive"],
                [{
                    "reference": _reference("muskingum-current-in"),
                    "source": _fact("incoming", "muskingum"),
                }],
                ["muskingum-c0", "muskingum-c1", "muskingum-c2"],
                [muskingum_update],
                0,
            ),
            _bounded_lag("travel-time-lag", "incoming", "lag", 2),
            _finite_recurrence(
                "linear-reservoir-state",
                ["extensive", "extensive"],
                [{
                    "reference": _reference("linear-previous-storage"),
                    "source": {
                        "kind": "previous_state",
                        "index": 0,
                        "value_kind": "extensive",
                    },
                }],
                ["linear-coefficient"],
                [linear_storage, linear_release],
                1,
            ),
        ],
        "initial_states": [
            {
                "projection": "linear-reservoir-state",
                "values": [
                    {"kind": "extensive", "value": 8.0},
                    {"kind": "extensive", "value": 0.0},
                ],
            },
            {
                "projection": "muskingum-in-lag",
                "values": [{"kind": "extensive", "value": 4.0}],
            },
            {
                "projection": "muskingum-out-lag",
                "values": [{"kind": "extensive", "value": 2.0}],
            },
            {
                "projection": "muskingum-routed",
                "values": [{"kind": "extensive", "value": 0.0}],
            },
            {
                "projection": "travel-time-lag",
                "values": [
                    {"kind": "extensive", "value": 3.0},
                    {"kind": "extensive", "value": 2.0},
                ],
            },
        ],
    }


def _rules() -> list[dict]:
    evaporation = incidence.table_lookup(
        "evaporation-by-temperature", incidence.forcing("air-temperature")
    )
    seepage = incidence.min(
        incidence.input("seepage-available"),
        incidence.max(
            incidence.literal(0.0),
            incidence.mul(
                incidence.param("seepage-conductance"),
                incidence.input("hydraulic-head"),
            ),
        ),
    )
    policy = incidence.clamp(
        incidence.select(
            incidence.compare(
                "greater_than_or_equal",
                incidence.input("policy-storage"),
                incidence.param("flood-threshold"),
            ),
            incidence.param("flood-release"),
            incidence.param("conservation-release"),
        ),
        incidence.literal(0.0),
        incidence.input("policy-available"),
    )
    demand = incidence.min(
        incidence.forcing("requested-demand"),
        _expression_node(
            "divide",
            incidence.input("demand-available"),
            incidence.param("delivery-period"),
        ),
    )

    def authored_rule(compartment: str, expression: dict, parameters=()) -> dict:
        return incidence.rule(
            compartment,
            "water",
            expression,
            incidence.release_all(f"{compartment}-out"),
            parameters,
        )

    return [
        authored_rule(
            "muskingum",
            incidence.projection("muskingum-routed"),
            {"muskingum-c0": 0.4, "muskingum-c1": 0.3, "muskingum-c2": 0.3},
        ),
        authored_rule("lag", incidence.projection("travel-time-lag")),
        authored_rule(
            "linear-reservoir",
            incidence.projection("linear-reservoir-state"),
            {"linear-coefficient": 0.25},
        ),
        authored_rule("evaporation", evaporation),
        authored_rule("seepage", seepage, {"seepage-conductance": 0.5}),
        authored_rule(
            "reservoir-policy",
            policy,
            {
                "flood-threshold": 50.0,
                "flood-release": 8.0,
                "conservation-release": 3.0,
            },
        ),
        authored_rule("demand", demand, {"delivery-period": 2.0}),
    ]


def hydrology_model_document() -> dict:
    forcings = {
        "air-temperature": [0.0, 10.0, 20.0],
        "seepage-available": [10.0, 10.0, 10.0],
        "hydraulic-head": [4.0, 6.0, 8.0],
        "policy-storage": [40.0, 60.0, 45.0],
        "policy-available": [10.0, 10.0, 10.0],
        "requested-demand": [6.0, 8.0, 10.0],
        "demand-available": [20.0, 20.0, 20.0],
    }
    input_bindings = (
        ("seepage", "seepage-available", "seepage-available"),
        ("seepage", "hydraulic-head", "hydraulic-head"),
        ("reservoir-policy", "policy-storage", "policy-storage"),
        ("reservoir-policy", "policy-available", "policy-available"),
        ("demand", "demand-available", "demand-available"),
    )
    return incidence.model_document(
        finite_compartments=list(RULE_COMPARTMENTS),
        boundary_accounts=["outside"],
        connections=[
            {"source": owner, "target": "outside"} for owner in RULE_COMPARTMENTS
        ],
        substances=["water"],
        initial_stocks=[
            {
                "compartment": owner,
                "amounts": [{"substance": "water", "amount": 100.0}],
            }
            for owner in RULE_COMPARTMENTS
        ],
        calendar={"origin_unix_seconds": 0, "timestep_seconds": 86_400},
        horizon={"first": 0, "last": 2},
        projections=_projection_document(),
        forcings=[
            {
                "id": identifier,
                "horizon": {"first": 0, "last": 2},
                "values": values,
            }
            for identifier, values in forcings.items()
        ],
        interpolation_tables=[{
            "id": "evaporation-by-temperature",
            "numerical_semantics_version": "v1",
            "boundary_policy": "clamp_to_endpoint",
            "abscissae": [0.0, 10.0, 20.0],
            "ordinates": [1.0, 2.0, 3.0],
        }],
        rules=_rules(),
        transfer_bindings=[
            {
                "compartment": owner,
                "substance": "water",
                "branch": f"{owner}-out",
                "destination": "outside",
            }
            for owner in RULE_COMPARTMENTS
        ],
        input_bindings=[
            {
                "compartment": owner,
                "substance": "water",
                "input": input_id,
                "value_kind": "scalar",
                "source": {"kind": "forcing", "forcing": forcing_id},
            }
            for owner, input_id, forcing_id in input_bindings
        ],
        units=[{"substance": "water", "unit": "m3", "quantum": 1.0e-6}],
    )


def test_python_authored_fixture_matches_rust_fixture_log_and_replays() -> None:
    document = hydrology_model_document()
    assert json.loads(json.dumps(document)) == document

    completed = incidence.compile_model(document).run(RUN_ID)
    canonical_bytes, digest = completed.authoritative_log()
    rust_fixture_bytes = Path(__file__).with_name(
        "hydrology_fixture_rust_log.bin"
    ).read_bytes()

    assert canonical_bytes == rust_fixture_bytes
    assert digest == RUST_LOG_DIGEST
    assert hashlib.sha256(canonical_bytes).hexdigest() == digest
    # authoritative_log() performs core replay against this completed run's own artifact.
    assert completed.authoritative_log() == (canonical_bytes, digest)

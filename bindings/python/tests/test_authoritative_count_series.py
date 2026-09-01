from __future__ import annotations

import incidence


INDIVIDUAL_COUNT = 4_394_222_044_288_838
MERGED_COUNT = 8_788_444_088_577_676
QUANTUM = 1.0e-6


def projection_spec(identifier: str, compartment: str) -> dict:
    return {
        "rule_ir_version": "v1",
        "numerical_semantics_version": "v1",
        "id": identifier,
        "value_kind": "extensive",
        "spec": {
            "kind": "ordered_rolling_aggregate",
            "source": {
                "kind": "authoritative_fact",
                "selector": {
                    "kind": "incoming_transfer_amount",
                    "compartment": compartment,
                    "substance": "water",
                },
            },
            "window": 1,
            "aggregate": "sum_oldest_to_newest",
        },
    }


def merged_count_document() -> dict:
    sources = [f"source-{index}" for index in range(4)]
    aggregators = ["aggregator-a", "aggregator-b"]
    half_count = INDIVIDUAL_COUNT // 2
    half_value = half_count * QUANTUM
    rules = [
        incidence.rule(
            source,
            "water",
            incidence.literal(half_value),
            incidence.release_all(f"{source}-out"),
        )
        for source in sources
    ]
    for aggregator in aggregators:
        identifier = f"{aggregator}-incoming"
        rules.append(
            incidence.rule(
                aggregator,
                "water",
                incidence.projection(identifier, value_kind="extensive"),
                incidence.release_all(f"{aggregator}-out"),
            )
        )
    transfer_bindings = []
    for index, source in enumerate(sources):
        transfer_bindings.append(
            {
                "compartment": source,
                "substance": "water",
                "branch": f"{source}-out",
                "destination": aggregators[index // 2],
            }
        )
    for aggregator in aggregators:
        transfer_bindings.append(
            {
                "compartment": aggregator,
                "substance": "water",
                "branch": f"{aggregator}-out",
                "destination": "sink",
            }
        )
    return incidence.model_document(
        finite_compartments=sources + aggregators,
        boundary_accounts=["sink"],
        connections=[
            {"source": source, "target": aggregators[index // 2]}
            for index, source in enumerate(sources)
        ]
        + [{"source": aggregator, "target": "sink"} for aggregator in aggregators],
        substances=["water"],
        initial_stocks=[
            {
                "compartment": source,
                "amounts": [{"substance": "water", "amount": half_value}],
            }
            for source in sources
        ],
        calendar={"origin_unix_seconds": 0, "timestep_seconds": 1},
        horizon={"first": 0, "last": 0},
        projections={
            "specifications": [
                projection_spec(f"{aggregator}-incoming", aggregator)
                for aggregator in aggregators
            ],
            "initial_states": [
                {"projection": f"{aggregator}-incoming", "values": []}
                for aggregator in aggregators
            ],
        },
        forcings=[],
        interpolation_tables=[],
        rules=rules,
        transfer_bindings=transfer_bindings,
        input_bindings=[],
        units=[{"substance": "water", "unit": "m3", "quantum": QUANTUM}],
    )


def test_completed_run_reads_merged_count_without_f64_redecoding() -> None:
    run = incidence.compile_model(merged_count_document()).run(bytes([0x62]) * 16)

    public = run.transfer_series("sink", "water", direction="incoming")
    counts = run.transfer_count_series("sink", "water", direction="incoming")

    assert public.values == [8_788_444_088.577_675]
    assert int(public.values[0] / QUANTUM) != MERGED_COUNT
    assert counts.presence == public.presence == ["present"]
    assert counts.values == [MERGED_COUNT]
    assert counts.values[0] == 2 * INDIVIDUAL_COUNT
    assert run.authoritative_log()[1]

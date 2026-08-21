import incidence


def test_plain_data_expression_partition_runs_each_named_branch_amount():
    disposition = {
        "rule_ir_version": "v1",
        "numerical_semantics_version": "v1",
        "partition": {
            "kind": "expression_partition",
            "branches": [
                {"branch": "to-b", "expression": incidence.literal(3.0)},
                {
                    "branch": "to-a",
                    "expression": incidence.add(
                        incidence.literal(1.0), incidence.literal(1.0)
                    ),
                },
            ],
        },
    }
    document = incidence.model_document(
        finite_compartments=["source"],
        boundary_accounts=["sink-a", "sink-b"],
        connections=[
            {"source": "source", "target": "sink-a"},
            {"source": "source", "target": "sink-b"},
        ],
        substances=["water"],
        initial_stocks=[
            {
                "compartment": "source",
                "amounts": [{"substance": "water", "amount": 10.0}],
            }
        ],
        calendar={"origin_unix_seconds": 0, "timestep_seconds": 1},
        horizon={"first": 0, "last": 0},
        projections={"specifications": [], "initial_states": []},
        forcings=[],
        interpolation_tables=[],
        rules=[
            incidence.rule(
                "source", "water", incidence.literal(10.0), disposition
            )
        ],
        transfer_bindings=[
            {
                "compartment": "source",
                "substance": "water",
                "branch": "to-a",
                "destination": "sink-a",
            },
            {
                "compartment": "source",
                "substance": "water",
                "branch": "to-b",
                "destination": "sink-b",
            },
        ],
        input_bindings=[],
        units=[{"substance": "water", "unit": "m3"}],
    )

    run = incidence.compile_model(document).run(bytes([0x31]) * 16)
    sink_a = run.transfer_series("sink-a", "water", direction="incoming")
    sink_b = run.transfer_series("sink-b", "water", direction="incoming")
    source = run.transfer_series("source", "water", direction="outgoing")

    assert sink_a.presence == ["present"]
    assert sink_a.values == [2.0]
    assert sink_b.presence == ["present"]
    assert sink_b.values == [3.0]
    assert source.presence == ["present"]
    assert source.values == [5.0]

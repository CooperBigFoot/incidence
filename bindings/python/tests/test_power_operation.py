import incidence


def _model(expression):
    return incidence.model_document(
        finite_compartments=["source"],
        boundary_accounts=["outside"],
        connections=[{"source": "source", "target": "outside"}],
        substances=["water"],
        initial_stocks=[{
            "compartment": "source",
            "amounts": [{"substance": "water", "amount": 10.0}],
        }],
        calendar={"origin_unix_seconds": 0, "timestep_seconds": 1},
        horizon={"first": 0, "last": 0},
        projections={"specifications": [], "initial_states": []},
        forcings=[],
        interpolation_tables=[],
        rules=[incidence.rule(
            "source", "water", expression, incidence.release_all("out")
        )],
        transfer_bindings=[{
            "compartment": "source",
            "substance": "water",
            "branch": "out",
            "destination": "outside",
        }],
        input_bindings=[],
        units=[{"substance": "water", "unit": "m3", "quantum": 1.0e-6}],
    )


def test_power_operation_is_public_plain_data_and_runs():
    expression = incidence.power(incidence.literal(6.25), incidence.literal(0.5))

    assert expression["expression"]["kind"] == "power"
    assert incidence.roundtrip_expression(expression) == expression

    run = incidence.compile_model(_model(expression)).run(bytes([0x50]) * 16)
    series = run.transfer_series("source", "water")
    assert series.presence == ["present"]
    assert series.values == [2.5]

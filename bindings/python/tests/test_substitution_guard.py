from __future__ import annotations

import incidence
import pytest

from benchmarks.sweep_baseline import build_model_document


def _held_model() -> incidence.CompiledModel:
    return incidence.compile_model(
        build_model_document(compartment_count=2, forcing_count=2, horizon_steps=3, parameter_value=0.2)
    )


def _run_unchanged(model: incidence.CompiledModel, marker: int) -> None:
    run = model.run(marker.to_bytes(16, "big"))
    assert run.model_digest == model.model_digest
    assert run.transfer_series("basin-00", "water").presence == ["present"] * 3


@pytest.mark.parametrize(
    "target,address",
    [
        ({"forcing": "forcing-00", "value": 0.4}, "forcing-00"),
        ({"connection": ["basin-00", "basin-01"], "value": 0.4}, "basin-00"),
        ({"initial_stock": ["basin-00", "water"], "value": 0.4}, "basin-00"),
        ({"calendar": "timestep_seconds", "value": 1.0}, "timestep_seconds"),
    ],
)
def test_non_parameter_targets_are_refused_without_changing_held_model(
    target: dict, address: str
) -> None:
    model = _held_model()
    digest = model.model_digest

    with pytest.raises(ValueError) as caught:
        model.run(bytes(16), substitutions=[target])

    message = str(caught.value)
    assert address in message
    assert "not substitutable" in message
    assert model.model_digest == digest
    _run_unchanged(model, 1)


def test_unknown_parameter_is_refused_without_changing_held_model() -> None:
    model = _held_model()

    with pytest.raises(ValueError, match="not-declared.*not declared.*not substitutable"):
        model.run(
            bytes(16),
            substitutions=[{
                "compartment": "basin-00",
                "substance": "water",
                "parameter": "not-declared",
                "value": 0.4,
            }],
        )

    _run_unchanged(model, 2)


def test_a_batch_with_one_invalid_target_is_atomic() -> None:
    model = _held_model()
    digest = model.model_digest
    valid = {
        "compartment": "basin-00",
        "substance": "water",
        "parameter": "release-coefficient",
        "value": 0.4,
    }
    invalid = {**valid, "parameter": "forcing-00"}

    with pytest.raises(ValueError, match="forcing-00.*not substitutable"):
        model.run(bytes(16), substitutions=[valid, invalid])

    assert model.model_digest == digest
    _run_unchanged(model, 3)

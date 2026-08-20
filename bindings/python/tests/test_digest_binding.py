from __future__ import annotations

import incidence
import pytest

from benchmarks.sweep_baseline import build_model_document


def substitution(value: float) -> list[dict[str, object]]:
    return [{
        "compartment": "basin-00",
        "substance": "water",
        "parameter": "release-coefficient",
        "value": value,
    }]


def test_substituted_runs_have_bound_distinct_digests() -> None:
    model = incidence.compile_model(
        build_model_document(0.2, compartment_count=2, forcing_count=2, horizon_steps=3)
    )

    run_id = bytes(16)
    first = model.run(run_id, substitutions=substitution(0.3))
    second = model.run(run_id, substitutions=substitution(0.4))

    assert first.model_digest != second.model_digest
    assert first.model_digest != model.model_digest
    with pytest.raises(ValueError, match="model digest mismatch"):
        first.replay_against(second)
    with pytest.raises(ValueError, match="model digest mismatch"):
        second.replay_against(first)
    first.replay_against(first)
    second.replay_against(second)

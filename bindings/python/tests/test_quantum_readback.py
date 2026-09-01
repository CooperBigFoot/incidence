from __future__ import annotations

import incidence

from benchmarks.sweep_baseline import build_model_document


def test_compiled_and_completed_models_read_back_the_declared_quantum() -> None:
    document = build_model_document(0.2, compartment_count=2, forcing_count=2, horizon_steps=3)
    document["units"][0]["quantum"] = 0.25
    model = incidence.compile_model(document)
    run = model.run(bytes(16))

    assert model.quantum("water") == 0.25
    assert run.quantum("water") == 0.25
    assert run.model_digest == model.model_digest

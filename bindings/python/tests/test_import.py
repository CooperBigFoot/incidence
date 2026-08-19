from __future__ import annotations

import json
from pathlib import Path

import incidence


def test_documented_entry_points_are_importable() -> None:
    expected = {
        "CompiledModel", "compile_model", "literal", "param", "input", "forcing",
        "projection", "add", "mul", "min", "max", "clamp", "compare", "select",
        "table_lookup", "roundtrip_expression", "model_document", "rule",
    }
    assert expected <= set(incidence.__all__)
    assert callable(incidence.compile_model)
    assert isinstance(incidence.CompiledModel, type)


def test_valid_document_compiles_to_an_opaque_core_model() -> None:
    document = json.loads(Path(__file__).with_name("fixture.json").read_text())
    model = incidence.compile_model(document)
    assert isinstance(model, incidence.CompiledModel)

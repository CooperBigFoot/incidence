from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any, Callable

import pytest

from incidence import compile_model


def fixture() -> dict[str, Any]:
    return json.loads(Path(__file__).with_name("fixture.json").read_text())


def nan_literal(document: dict[str, Any]) -> None:
    document["initial_stocks"][0]["amounts"][0]["amount"] = float("nan")


def infinite_literal(document: dict[str, Any]) -> None:
    document["initial_stocks"][0]["amounts"][0]["amount"] = float("inf")


def too_deep_expression(document: dict[str, Any]) -> None:
    leaf: dict[str, Any] = {"kind": "literal", "value": 1.0}
    expression = leaf
    for _ in range(129):
        expression = {"kind": "add", "lhs": leaf, "rhs": expression}
    document["rules"][0]["expression"]["expression"] = expression


def cyclic_topology(document: dict[str, Any]) -> None:
    document["connections"] = [
        {"source": "muskingum", "target": "lag"},
        {"source": "lag", "target": "muskingum"},
    ]


def unknown_substance(document: dict[str, Any]) -> None:
    document["initial_stocks"][0]["amounts"][0]["substance"] = "unknown-substance"


def unknown_compartment(document: dict[str, Any]) -> None:
    document["initial_stocks"][0]["compartment"] = "unknown-compartment"


def duplicate_compartment(document: dict[str, Any]) -> None:
    document["finite_compartments"].append(document["finite_compartments"][0])


def negative_initial_stock(document: dict[str, Any]) -> None:
    document["initial_stocks"][0]["amounts"][0]["amount"] = -1.0


MALFORMED: tuple[tuple[str, Callable[[dict[str, Any]], None], str], ...] = (
    ("nan", nan_literal, "non-negative amount must be finite, got nan"),
    ("infinite", infinite_literal, "non-negative amount must be finite, got inf"),
    ("deep expression", too_deep_expression, "rule expression depth 129 exceeds maximum 128"),
    ("cyclic topology", cyclic_topology, "topology"),
    ("unknown substance", unknown_substance, "substance"),
    ("unknown compartment", unknown_compartment, "compartment"),
    ("duplicate compartment", duplicate_compartment, "topology"),
    ("negative initial stock", negative_initial_stock, "non-negative amount cannot be negative"),
)


def test_all_malformed_documents_are_ordinary_exceptions_in_one_process() -> None:
    caught: list[Exception] = []
    for name, mutate, expected_text in MALFORMED:
        document = copy.deepcopy(fixture())
        mutate(document)
        try:
            compile_model(document)
        except Exception as error:
            caught.append(error)
            assert expected_text in str(error).lower(), name
            assert type(error).__module__ != "pyo3_runtime", name
        else:
            pytest.fail(f"{name} unexpectedly produced an artifact")

    assert len(caught) == len(MALFORMED)

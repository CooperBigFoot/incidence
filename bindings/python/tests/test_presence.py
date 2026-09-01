from __future__ import annotations

import copy
import json
from pathlib import Path

import incidence


def dry_model_document() -> dict:
    document = copy.deepcopy(
        json.loads(Path(__file__).with_name("fixture.json").read_text())
    )
    document["horizon"] = {"first": 1, "last": 40}
    for forcing in document["forcings"]:
        forcing["horizon"] = {"first": 1, "last": 40}
        forcing["values"] = [forcing["values"][0]] * 40
    for rule in document["rules"]:
        rule["expression"]["expression"] = {"kind": "literal", "value": 0.0}
        rule["parameters"] = []
    document["input_bindings"] = []
    return document


def test_dry_timesteps_and_absence_remain_distinct() -> None:
    run = incidence.compile_model(dry_model_document()).run(bytes(range(16)))

    series = run.transfer_series(
        "demand", "water", direction="outgoing", first=0, last=41
    )

    assert isinstance(run, incidence.CompletedRun)
    assert isinstance(series, incidence.PresenceSeries)
    assert series.timesteps == list(range(42))
    assert series.presence == ["absent"] + ["present"] * 40 + ["absent"]
    assert series.values == [None] + [0.0] * 40 + [None]


def test_undeclared_substance_never_becomes_zero() -> None:
    run = incidence.compile_model(dry_model_document()).run(
        "00010203-0405-0607-0809-0a0b0c0d0e0f"
    )

    series = run.transfer_series(
        "demand", "salt", direction="outgoing", first=0, last=41
    )

    assert series.presence == ["not_modelled"] * 42
    assert series.values == [None] * 42
    assert 0.0 not in series.values


def test_authoritative_count_series_retains_zero_absence_and_not_modelled() -> None:
    run = incidence.compile_model(dry_model_document()).run(bytes([0x44]) * 16)

    modelled = run.transfer_count_series(
        "demand", "water", direction="outgoing", first=0, last=41
    )
    not_modelled = run.transfer_count_series(
        "demand", "salt", direction="outgoing", first=0, last=41
    )

    assert isinstance(modelled, incidence.PresenceCountSeries)
    assert modelled.presence == ["absent"] + ["present"] * 40 + ["absent"]
    assert modelled.values == [None] + [0] * 40 + [None]
    assert not_modelled.presence == ["not_modelled"] * 42
    assert not_modelled.values == [None] * 42

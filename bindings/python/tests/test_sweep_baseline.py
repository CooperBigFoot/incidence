from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

from benchmarks.sweep_baseline import (
    COMPARTMENT_COUNT,
    FORCING_COUNT,
    HORIZON_STEPS,
    TRIAL_COUNT,
    build_model_document,
    load_record,
    measure_trial,
)

RECORD = Path(__file__).parents[1] / "benchmarks" / "sweep-baseline-v1.json"


def test_basin_workload_has_the_committed_scale() -> None:
    document = build_model_document(0.25)

    assert len(document["finite_compartments"]) == COMPARTMENT_COUNT == 50
    assert len(document["forcings"]) == FORCING_COUNT == 20
    assert document["horizon"] == {"first": 0, "last": HORIZON_STEPS - 1}
    assert all(len(forcing["values"]) == HORIZON_STEPS == 2_191 for forcing in document["forcings"])
    assert document["rules"][0]["parameters"] == [
        {"id": "release-coefficient", "value": 0.25}
    ]


def test_committed_record_is_readable_by_the_held_model_package() -> None:
    record = load_record(RECORD)

    assert {key: record["workload"][key] for key in (
        "compartments", "forcing_series", "forcing_values_per_trial",
        "full_document_submissions", "horizon_steps", "parameter_varied",
        "presence_reads", "runs", "trials",
    )} == {
        "compartments": 50,
        "forcing_series": 20,
        "forcing_values_per_trial": 43_820,
        "full_document_submissions": TRIAL_COUNT,
        "horizon_steps": 2_191,
        "parameter_varied": "basin-00.release-coefficient",
        "presence_reads": TRIAL_COUNT,
        "runs": TRIAL_COUNT,
        "trials": TRIAL_COUNT,
    }
    document_bytes = json.dumps(
        build_model_document(0.2), sort_keys=True, separators=(",", ":")
    ).encode()
    assert record["workload"]["base_document_json_bytes"] == len(document_bytes)
    assert record["workload"]["base_document_sha256"] == hashlib.sha256(
        document_bytes
    ).hexdigest()
    assert record["procedure"]["build_profile"] == "release"
    assert "--trials 1000 --workers 12" in record["procedure"]["reproduction_command"]
    source_revision = record["procedure"]["source_revision"]
    assert len(source_revision) == 40
    benchmark_at_source = subprocess.run(
        [
            "git",
            "cat-file",
            "-e",
            f"{source_revision}:bindings/python/benchmarks/sweep_baseline.py",
        ],
        cwd=Path(__file__).parents[3],
        check=False,
    )
    assert benchmark_at_source.returncode == 0
    assert record["execution"]["wall_clock_seconds"] > 0
    assert record["execution"]["workers"] >= 1
    assert set(record["machine"]) >= {"node", "platform", "machine", "logical_cpu_count"}
    assert set(record["toolchains"]) == {
        "cargo", "maturin", "python", "python_implementation", "rustc", "uv"
    }
    assert record["phases"]["decode_validate_seconds"]["mean_seconds"] > 0
    assert record["phases"]["trial_seconds"]["total_seconds"] > 0


def test_harness_compiles_runs_and_reads_presence() -> None:
    sample = measure_trial(
        0, compartment_count=2, forcing_count=2, horizon_steps=3
    )

    assert set(sample) == {
        "document_build_seconds",
        "decode_validate_seconds",
        "run_seconds",
        "presence_read_seconds",
        "trial_seconds",
    }
    assert all(duration >= 0 for duration in sample.values())

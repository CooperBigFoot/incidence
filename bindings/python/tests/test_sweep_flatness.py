from __future__ import annotations

from pathlib import Path
import subprocess

import incidence

from benchmarks.sweep_baseline import TRIAL_COUNT, build_model_document, load_record
from benchmarks.sweep_held_model import load_held_record, measure_trial

ROOT = Path(__file__).parents[1]
BASELINE = ROOT / "benchmarks" / "sweep-baseline-v1.json"
HELD = ROOT / "benchmarks" / "sweep-held-model-v1.json"


def test_committed_comparison_reports_flat_decode_and_validate_work() -> None:
    baseline = load_record(BASELINE)
    held = load_held_record(HELD)

    assert held["workload"] == {
        "compartments": 50,
        "horizon_steps": 2_191,
        "forcing_series": 20,
        "trials": TRIAL_COUNT,
        "parameter_varied": "basin-00.release-coefficient",
        "full_document_submissions": 1,
        "full_document_decode_validate_operations": 1,
        "forcing_values_crossed_initially": 43_820,
        "forcing_values_crossed_after_compile": 0,
        "parameter_submissions": TRIAL_COUNT,
        "runs": TRIAL_COUNT,
        "presence_reads": TRIAL_COUNT,
    }
    comparison = held["comparison"]
    assert comparison["naive_full_document_submissions"] == TRIAL_COUNT
    assert comparison["held_full_document_submissions"] == 1
    assert comparison["naive_full_document_decode_validate_operations"] == TRIAL_COUNT
    assert comparison["held_full_document_decode_validate_operations"] == 1
    assert comparison["naive_decode_validate_total_seconds"] == baseline["phases"]["decode_validate_seconds"]["total_seconds"]
    assert comparison["held_decode_validate_total_seconds"] == held["initial_decode_validate_seconds"]
    assert comparison["naive_wall_clock_seconds"] == baseline["execution"]["wall_clock_seconds"]
    assert comparison["held_wall_clock_seconds"] == held["execution"]["wall_clock_seconds"]
    assert comparison["naive_trial_total_seconds"] == baseline["phases"]["trial_seconds"]["total_seconds"]
    assert comparison["held_trial_total_seconds"] == held["phases"]["trial_seconds"]["total_seconds"]
    assert comparison["naive_run_total_seconds"] == baseline["phases"]["run_seconds"]["total_seconds"]
    assert comparison["held_parameter_run_total_seconds"] == held["phases"]["parameter_run_seconds"]["total_seconds"]
    assert held["machine"] == baseline["machine"]
    assert held["execution"]["workers"] == baseline["execution"]["workers"]
    source_revision = held["procedure"]["source_revision"]
    assert len(source_revision) == 40
    benchmark_at_source = subprocess.run(
        ["git", "cat-file", "-e", f"{source_revision}:bindings/python/benchmarks/sweep_held_model.py"],
        cwd=Path(__file__).parents[3],
        check=False,
    )
    assert benchmark_at_source.returncode == 0


def test_small_held_sweep_sends_only_parameter_data_after_compile() -> None:
    model = incidence.compile_model(
        build_model_document(0.2, compartment_count=2, forcing_count=2, horizon_steps=3)
    )

    sample = measure_trial(model, 1, horizon_steps=3)

    assert set(sample) == {
        "parameter_run_seconds", "presence_read_seconds", "trial_seconds"
    }
    assert all(duration >= 0 for duration in sample.values())


def test_committed_record_names_the_code_that_can_reproduce_it() -> None:
    held = load_held_record(HELD)
    source_revision = held["procedure"]["source_revision"]
    benchmark_path = "bindings/python/benchmarks/sweep_held_model.py"
    benchmark_at_source = subprocess.run(
        ["git", "show", f"{source_revision}:{benchmark_path}"],
        cwd=Path(__file__).parents[3],
        check=True,
        capture_output=True,
    ).stdout

    assert benchmark_at_source == (ROOT / "benchmarks" / "sweep_held_model.py").read_bytes()

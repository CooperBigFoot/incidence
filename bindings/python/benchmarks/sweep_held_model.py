"""Measure parameter-only runs from one held, validated model.

The full basin document crosses the Python boundary once. Every trial after that sends only one
rule-parameter substitution and a run id. Execution releases the GIL, so the worker count can
match the committed process-parallel naive baseline without copying the held model.
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import incidence

from benchmarks.sweep_baseline import (
    COMPARTMENT_COUNT,
    FORCING_COUNT,
    HORIZON_STEPS,
    TRIAL_COUNT,
    build_model_document,
    load_record,
    machine_and_toolchains,
)

RECORD_SCHEMA = "incidence.sweep-held-model.v1"
BASELINE = Path(__file__).with_name("sweep-baseline-v1.json")


def substitution(value: float) -> list[dict[str, object]]:
    """Return the only per-trial model data allowed to cross the boundary."""
    return [{
        "compartment": "basin-00",
        "substance": "water",
        "parameter": "release-coefficient",
        "value": value,
    }]


def measure_trial(
    model: incidence.CompiledModel,
    trial: int,
    *,
    horizon_steps: int = HORIZON_STEPS,
) -> dict[str, float]:
    """Substitute, execute, and read presence without resending model components."""
    started = time.perf_counter()
    value = 0.2 + (trial % 600) / 1_000.0
    run = model.run(
        trial.to_bytes(16, byteorder="big", signed=False),
        substitutions=substitution(value),
    )
    run_seconds = time.perf_counter() - started
    read_started = time.perf_counter()
    series = run.transfer_series("basin-00", "water", direction="outgoing")
    read_seconds = time.perf_counter() - read_started
    if len(series.presence) != horizon_steps or any(
        state != "present" for state in series.presence
    ):
        raise RuntimeError("presence-carrying result did not cover the declared horizon")
    return {
        "parameter_run_seconds": run_seconds,
        "presence_read_seconds": read_seconds,
        "trial_seconds": time.perf_counter() - started,
    }


def _summary(samples: list[float]) -> dict[str, float]:
    ordered = sorted(samples)
    p95_index = max(0, math.ceil(0.95 * len(ordered)) - 1)
    return {
        "total_seconds": sum(samples),
        "mean_seconds": statistics.fmean(samples),
        "median_seconds": statistics.median(samples),
        "p95_seconds": ordered[p95_index],
        "minimum_seconds": ordered[0],
        "maximum_seconds": ordered[-1],
    }


def _revision() -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()


def run_held_sweep(
    *, trials: int = TRIAL_COUNT, workers: int = 1, baseline_path: Path = BASELINE
) -> dict[str, Any]:
    """Measure one compile followed by parameter-only trials and compare the baseline."""
    if trials < 1:
        raise ValueError("trials must be positive")
    if workers < 1:
        raise ValueError("workers must be positive")
    baseline = load_record(baseline_path)
    document = build_model_document(0.2)
    decode_started = time.perf_counter()
    model = incidence.compile_model(document)
    decode_seconds = time.perf_counter() - decode_started

    wall_started = time.perf_counter()
    if workers == 1:
        samples = [measure_trial(model, trial) for trial in range(trials)]
    else:
        with ThreadPoolExecutor(max_workers=workers) as executor:
            samples = list(executor.map(lambda trial: measure_trial(model, trial), range(trials)))
    wall_seconds = time.perf_counter() - wall_started
    phases = {
        phase: _summary([sample[phase] for sample in samples])
        for phase in ("parameter_run_seconds", "presence_read_seconds", "trial_seconds")
    }
    baseline_decode = baseline["phases"]["decode_validate_seconds"]["total_seconds"]
    return {
        "schema": RECORD_SCHEMA,
        "recorded_at_utc": datetime.now(UTC).isoformat(),
        "procedure": {
            "build_profile": "release",
            "source_revision": _revision(),
            "baseline_schema": baseline["schema"],
            "baseline_source_revision": baseline["procedure"]["source_revision"],
            "reproduction_command": (
                "uv run --no-sync maturin develop --uv --release -q && "
                "PYTHONPATH=. uv run --no-sync python -m "
                f"benchmarks.sweep_held_model --trials {trials} --workers {workers}"
            ),
        },
        "workload": {
            "compartments": COMPARTMENT_COUNT,
            "horizon_steps": HORIZON_STEPS,
            "forcing_series": FORCING_COUNT,
            "trials": trials,
            "parameter_varied": "basin-00.release-coefficient",
            "full_document_submissions": 1,
            "decode_validate_operations": 1,
            "forcing_values_crossed_initially": FORCING_COUNT * HORIZON_STEPS,
            "forcing_values_crossed_after_compile": 0,
            "parameter_submissions": trials,
            "runs": trials,
            "presence_reads": trials,
        },
        "execution": {"workers": workers, "wall_clock_seconds": wall_seconds},
        "initial_decode_validate_seconds": decode_seconds,
        "phases": phases,
        "comparison": {
            "naive_full_document_submissions": baseline["workload"]["full_document_submissions"],
            "held_full_document_submissions": 1,
            "naive_decode_validate_total_seconds": baseline_decode,
            "held_decode_validate_total_seconds": decode_seconds,
            "naive_decode_validate_operations": baseline["workload"]["trials"],
            "held_decode_validate_operations": 1,
        },
        **machine_and_toolchains(),
    }


def load_held_record(path: Path) -> dict[str, Any]:
    """Load and minimally validate a held-model comparison record."""
    record = json.loads(path.read_text())
    if record.get("schema") != RECORD_SCHEMA:
        raise ValueError(f"unsupported held-model record schema: {record.get('schema')!r}")
    for section in (
        "procedure", "workload", "execution", "initial_decode_validate_seconds",
        "phases", "comparison", "machine", "toolchains",
    ):
        if section not in record:
            raise ValueError(f"held-model record is missing {section!r}")
    return record


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trials", type=int, default=TRIAL_COUNT)
    parser.add_argument("--workers", type=int, default=1)
    parser.add_argument(
        "--baseline", type=Path, default=BASELINE
    )
    parser.add_argument(
        "--output", type=Path,
        default=Path(__file__).with_name("sweep-held-model-v1.json"),
    )
    args = parser.parse_args()
    record = run_held_sweep(
        trials=args.trials, workers=args.workers, baseline_path=args.baseline
    )
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
    temporary.replace(args.output)
    print(args.output)


if __name__ == "__main__":
    main()

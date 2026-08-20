"""Measure whole-document sweep cost through the public Python boundary.

Each trial constructs and submits the complete document, compiles it in the core, runs it,
and reads one presence-carrying result. The committed record is the comparison input for IPB7.
Run this against a release extension; debug execution is intentionally much slower.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import statistics
import subprocess
import sys
import time
from concurrent.futures import ProcessPoolExecutor
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import incidence

COMPARTMENT_COUNT = 50
FORCING_COUNT = 20
HORIZON_STEPS = 2_191
TRIAL_COUNT = 1_000
RECORD_SCHEMA = "incidence.sweep-baseline.v1"


def build_model_document(
    parameter_value: float,
    *,
    compartment_count: int = COMPARTMENT_COUNT,
    forcing_count: int = FORCING_COUNT,
    horizon_steps: int = HORIZON_STEPS,
) -> dict[str, Any]:
    """Build one complete, independent basin-scale plain-data document."""
    compartments = [f"basin-{index:02d}" for index in range(compartment_count)]
    horizon = {"first": 0, "last": horizon_steps - 1}
    return incidence.model_document(
        finite_compartments=compartments,
        boundary_accounts=["outside"],
        connections=[],
        substances=["water"],
        initial_stocks=[
            {
                "compartment": compartments[0],
                "amounts": [{"substance": "water", "amount": 1.0}],
            }
        ],
        calendar={"origin_unix_seconds": 0, "timestep_seconds": 86_400},
        horizon=horizon,
        projections={"specifications": [], "initial_states": []},
        forcings=[
            {
                "id": f"forcing-{index:02d}",
                "horizon": dict(horizon),
                "values": [float(index)] * horizon_steps,
            }
            for index in range(forcing_count)
        ],
        interpolation_tables=[],
        rules=[
            incidence.rule(
                compartments[0],
                "water",
                incidence.param("release-coefficient"),
                incidence.retain_all(),
                {"release-coefficient": parameter_value},
            )
        ],
        transfer_bindings=[],
        input_bindings=[],
        units=[{"substance": "water", "unit": "m3"}],
    )


def measure_trial(
    trial: int,
    *,
    compartment_count: int = COMPARTMENT_COUNT,
    forcing_count: int = FORCING_COUNT,
    horizon_steps: int = HORIZON_STEPS,
) -> dict[str, float]:
    """Submit, compile, run, and read one independently authored document."""
    trial_started = time.perf_counter()
    parameter_value = 0.2 + (trial % 600) / 1_000.0
    document = build_model_document(
        parameter_value,
        compartment_count=compartment_count,
        forcing_count=forcing_count,
        horizon_steps=horizon_steps,
    )
    decode_started = time.perf_counter()
    model = incidence.compile_model(document)
    decode_seconds = time.perf_counter() - decode_started

    run_id = trial.to_bytes(16, byteorder="big", signed=False)
    run_started = time.perf_counter()
    completed = model.run(run_id)
    run_seconds = time.perf_counter() - run_started

    read_started = time.perf_counter()
    series = completed.transfer_series(
        "basin-00", "water", direction="outgoing"
    )
    read_seconds = time.perf_counter() - read_started
    if len(series.presence) != horizon_steps or any(
        state != "present" for state in series.presence
    ):
        raise RuntimeError("presence-carrying result did not cover the declared horizon")
    return {
        "document_build_seconds": decode_started - trial_started,
        "decode_validate_seconds": decode_seconds,
        "run_seconds": run_seconds,
        "presence_read_seconds": read_seconds,
        "trial_seconds": time.perf_counter() - trial_started,
    }


def _measure_trial_number(trial: int) -> dict[str, float]:
    return measure_trial(trial)


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


def _version(command: list[str]) -> str:
    result = subprocess.run(command, check=True, capture_output=True, text=True)
    return (result.stdout or result.stderr).strip().splitlines()[0]


def machine_and_toolchains() -> dict[str, Any]:
    """Return enough environment identity to reproduce and compare the record."""
    return {
        "machine": {
            "node": platform.node(),
            "platform": platform.platform(),
            "machine": platform.machine(),
            "processor": platform.processor(),
            "logical_cpu_count": os.cpu_count(),
        },
        "toolchains": {
            "python": sys.version.splitlines()[0],
            "python_implementation": platform.python_implementation(),
            "rustc": _version(["rustc", "--version"]),
            "cargo": _version(["cargo", "--version"]),
            "uv": _version(["uv", "--version"]),
            "maturin": _version([sys.executable, "-m", "maturin", "--version"]),
        },
    }


def run_baseline(*, trials: int = TRIAL_COUNT, workers: int = 1) -> dict[str, Any]:
    """Run the naive full-document baseline and return its portable record."""
    if trials < 1:
        raise ValueError("trials must be positive")
    if workers < 1:
        raise ValueError("workers must be positive")
    wall_started = time.perf_counter()
    if workers == 1:
        samples = [_measure_trial_number(trial) for trial in range(trials)]
    else:
        with ProcessPoolExecutor(max_workers=workers) as executor:
            samples = list(executor.map(_measure_trial_number, range(trials)))
    wall_seconds = time.perf_counter() - wall_started
    phases = {
        phase: _summary([sample[phase] for sample in samples])
        for phase in (
            "document_build_seconds",
            "decode_validate_seconds",
            "run_seconds",
            "presence_read_seconds",
            "trial_seconds",
        )
    }
    fingerprint_document = build_model_document(0.2)
    fingerprint_bytes = json.dumps(
        fingerprint_document, sort_keys=True, separators=(",", ":")
    ).encode()
    return {
        "schema": RECORD_SCHEMA,
        "recorded_at_utc": datetime.now(UTC).isoformat(),
        "procedure": {
            "build_profile": "release",
            "source_revision": _version(["git", "rev-parse", "HEAD"]),
            "reproduction_command": (
                "uv run --no-sync maturin develop --uv --release -q && "
                "PYTHONPATH=. uv run --no-sync python -m "
                f"benchmarks.sweep_baseline --trials {trials} --workers {workers}"
            ),
        },
        "workload": {
            "compartments": COMPARTMENT_COUNT,
            "horizon_steps": HORIZON_STEPS,
            "forcing_series": FORCING_COUNT,
            "forcing_values_per_trial": FORCING_COUNT * HORIZON_STEPS,
            "trials": trials,
            "parameter_varied": "basin-00.release-coefficient",
            "full_document_submissions": trials,
            "runs": trials,
            "presence_reads": trials,
            "base_document_json_bytes": len(fingerprint_bytes),
            "base_document_sha256": hashlib.sha256(fingerprint_bytes).hexdigest(),
        },
        "execution": {"workers": workers, "wall_clock_seconds": wall_seconds},
        "phases": phases,
        **machine_and_toolchains(),
    }


def load_record(path: Path) -> dict[str, Any]:
    """Load and minimally validate a baseline record for the next package."""
    record = json.loads(path.read_text())
    if record.get("schema") != RECORD_SCHEMA:
        raise ValueError(f"unsupported baseline record schema: {record.get('schema')!r}")
    for section in (
        "procedure", "workload", "execution", "phases", "machine", "toolchains"
    ):
        if section not in record:
            raise ValueError(f"baseline record is missing {section!r}")
    return record


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trials", type=int, default=TRIAL_COUNT)
    parser.add_argument("--workers", type=int, default=1)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).with_name("sweep-baseline-v1.json"),
    )
    args = parser.parse_args()
    record = run_baseline(trials=args.trials, workers=args.workers)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
    temporary.replace(args.output)
    print(args.output)


if __name__ == "__main__":
    main()

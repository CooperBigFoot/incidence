# incidence Python binding

The binding accepts a complete plain-data model document and delegates all model validation to
`incidence-core`. It does not duplicate domain validation in Python.

```python
from incidence import compile_model

model = compile_model(document)  # `document` is a plain Python mapping
run = model.run(bytes.fromhex("000102030405060708090a0b0c0d0e0f"))
release = run.transfer_series("reservoir", "water", direction="outgoing")
assert len(release.timesteps) == len(release.values) == len(release.presence)
```

## Public entry points

- `compile_model(document)` decodes a plain-data Python value through Serde into the public Rust
  `ModelDocument`, then validates it into an opaque `CompiledModel`.
- `CompiledModel` holds the validated core artifact. `CompiledModel.run(run_id)` executes it and
  returns an opaque sealed `CompletedRun`.
- `CompletedRun.transfer_series(...)` returns a `PresenceSeries`. The result carries timestep,
  value, and presence arrays of equal length. Dry modelled timesteps contain `0.0` with
  `"present"`; positions outside the horizon contain `None` with `"absent"`; substances outside
  the model registry contain `None` with `"not_modelled"`.
- `CompletedRun.authoritative_log()` returns the authoritative log's canonical bytes and its
  lowercase hexadecimal digest as a tuple. The bytes are the exact payload authenticated by the
  core digest.

All Rust panics in exported operations are contained at this module boundary and converted to
`RuntimeError`. Decode and core validation failures are `ValueError`, so callers can safely
isolate bad models with `except Exception`.

## Reproducible development build

From `bindings/python`, the package driver can be run repeatedly without changing the command:

```console
uv venv --allow-existing --quiet
uv sync --quiet
uv run --no-sync maturin develop --uv -q
uv run --no-sync pytest tests/test_import.py -q
uv run --no-sync pytest tests/test_panic_containment.py -q
```

The project requires CPython 3.12 or newer. `uv.lock` pins the build and test tools.

## Plain-data authoring

Expression helpers such as `literal`, `param`, `input`, `forcing`, `projection`, `add`,
`mul`, `min`, `max`, `clamp`, `compare`, `select`, and `table_lookup` call the Rust IR
constructors and return only JSON-serialisable dictionaries. `model_document`, `rule`,
`retain_all`, and `release_all` assemble the surrounding document shape without duplicating
domain validation. `compile_model` remains the single validation entry point.

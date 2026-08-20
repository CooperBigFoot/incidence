from __future__ import annotations

from collections.abc import Sequence
import inspect

import incidence

from test_presence import dry_model_document


def is_numeric_sequence(value: object) -> bool:
    return (
        isinstance(value, Sequence)
        and not isinstance(value, (str, bytes, bytearray))
        and all(isinstance(item, (int, float)) for item in value)
    )


def test_complete_public_surface_has_no_bare_time_series() -> None:
    model = incidence.compile_model(dry_model_document())
    run = model.run(bytes(16))
    result = run.transfer_series("demand", "water")

    assert set(name for name in dir(run) if not name.startswith("_")) == {
        "model_digest", "replay_against", "transfer_series"
    }
    assert isinstance(result, incidence.PresenceSeries)
    assert len(result.timesteps) == len(result.values) == len(result.presence)
    assert set(result.presence) <= {"present", "absent", "not_modelled"}

    for name in incidence.__all__:
        value = getattr(incidence, name)
        assert not is_numeric_sequence(value), name

    public_methods = [
        name
        for name, member in inspect.getmembers(incidence.CompletedRun)
        if not name.startswith("_") and callable(member)
    ]
    assert public_methods == ["replay_against", "transfer_series"]
    assert run.replay_against(run) is None
    assert not is_numeric_sequence(result)

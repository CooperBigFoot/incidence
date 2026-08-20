from __future__ import annotations

import incidence

from test_presence import dry_model_document


def test_values_accessor_carries_its_presence_states() -> None:
    run = incidence.compile_model(dry_model_document()).run(bytes(16))

    series = run.transfer_series(
        "demand", "salt", direction="outgoing", first=0, last=41
    )
    values = series.values

    assert not isinstance(values, list)
    assert values == [None] * 42
    assert values.presence == ["not_modelled"] * 42
    assert len(values) == len(values.presence)

from __future__ import annotations

import hashlib

import incidence

from test_presence import dry_model_document


def test_completed_run_yields_authenticated_authoritative_log() -> None:
    run = incidence.compile_model(dry_model_document()).run(bytes(range(16)))

    canonical_bytes, digest = run.authoritative_log()

    assert isinstance(canonical_bytes, bytes)
    assert canonical_bytes.startswith(b"incidence:authoritative-log:v2\0")
    assert digest == hashlib.sha256(canonical_bytes).hexdigest()
    assert run.authoritative_log() == (canonical_bytes, digest)

    # The accessor verifies authoritative replay against the held artifact before returning.
    series = run.transfer_series("demand", "water")
    assert series.presence == ["present"] * 40

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def test_deeply_nested_document_is_rejected_without_crashing_python() -> None:
    fixture_path = Path(__file__).with_name("fixture.json")
    script = r"""
import json
import sys

from incidence import compile_model

with open(sys.argv[1]) as fixture_file:
    document = json.load(fixture_file)

leaf = {"kind": "literal", "value": 1.0}
expression = leaf
for _ in range(1_024):
    expression = {"kind": "add", "lhs": leaf, "rhs": expression}
document["rules"][0]["expression"]["expression"] = expression

try:
    compile_model(document)
except Exception as error:
    print(type(error).__name__)
else:
    raise SystemExit("deeply nested document unexpectedly compiled")
"""

    result = subprocess.run(
        [sys.executable, "-c", script, str(fixture_path)],
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "ValueError"

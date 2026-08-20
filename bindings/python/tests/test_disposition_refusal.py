import copy
import json
from pathlib import Path

import pytest

import incidence


def test_positive_stock_for_an_omitted_substance_is_refused_at_compile_time():
    fixture = json.loads((Path(__file__).parent / "fixture.json").read_text())
    document = copy.deepcopy(fixture)
    document["substances"].append("salt")
    document["units"].append({"substance": "salt", "unit": "mass"})
    document["initial_stocks"][0]["amounts"].append(
        {"substance": "salt", "amount": 3.0}
    )

    with pytest.raises(Exception) as raised:
        incidence.compile_model(document)

    message = str(raised.value)
    assert "muskingum" in message
    assert "salt" in message
    assert "timestep 0" in message
    assert not isinstance(raised.value, BaseException) or isinstance(raised.value, Exception)

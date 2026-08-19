import subprocess
import sys
import textwrap


def test_cyclic_expression_is_an_ordinary_python_exception():
    program = textwrap.dedent(
        """
        import incidence

        node = {"kind": "add"}
        node["lhs"] = node
        node["rhs"] = {"kind": "literal", "value": 1.0}
        expression = {
            "rule_ir_version": "v1",
            "numerical_semantics_version": "v1",
            "expression": node,
        }

        try:
            incidence.roundtrip_expression(expression)
        except Exception as error:
            assert "cyclic Python container" in str(error)
        else:
            raise AssertionError("cyclic expression was accepted")
        """
    )
    completed = subprocess.run(
        [sys.executable, "-c", program],
        check=False,
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr

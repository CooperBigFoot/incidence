import json

import incidence


def _assert_plain_roundtrip(expression):
    assert json.loads(json.dumps(expression)) == expression
    assert incidence.roundtrip_expression(expression) == expression

    def walk(value):
        assert not callable(value)
        if isinstance(value, dict):
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)

    walk(expression)


def test_every_expression_combinator_builds_plain_validated_data():
    one = incidence.literal(1.0)
    two = incidence.literal(2.0)
    parameter = incidence.param("coefficient")
    generic_input = incidence.input("available")
    forcing = incidence.forcing("rainfall")
    projection = incidence.projection("storage", "extensive")
    addition = incidence.add(one, parameter)
    multiplication = incidence.mul(addition, forcing)
    powered = incidence.power(multiplication, incidence.literal(0.5))
    minimum = incidence.min(powered, generic_input)
    maximum = incidence.max(minimum, projection)
    clamped = incidence.clamp(maximum, one, incidence.literal(100.0))
    condition = incidence.compare("greater_than", generic_input, two)
    selected = incidence.select(condition, clamped, one)
    lookup = incidence.table_lookup("rating-curve", selected)

    for expression in (
        one, parameter, generic_input, forcing, projection, addition,
        multiplication, powered, minimum, maximum, clamped, selected, lookup,
    ):
        _assert_plain_roundtrip(expression)


def test_document_helpers_only_assemble_plain_data():
    authored_rule = incidence.rule(
        "reach", "water", incidence.literal(0.0), incidence.release_all("out"),
        {"coefficient": 0.5},
    )
    document = incidence.model_document(rules=[authored_rule])
    assert json.loads(json.dumps(document)) == document

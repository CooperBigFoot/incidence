"""Plain-data Python boundary for the incidence conserved-flow engine."""

from . import _incidence

CompiledModel = _incidence.CompiledModel
CompletedRun = _incidence.CompletedRun
PresenceSeries = _incidence.PresenceSeries
PresenceCountSeries = _incidence.PresenceCountSeries
compile_model = _incidence.compile_model

literal = _incidence.literal
param = _incidence.param
parameter = param
input = _incidence.input
forcing = _incidence.forcing
projection = _incidence.projection
add = _incidence.add
mul = _incidence.mul
multiply = mul
power = _incidence.power
min = _incidence.min
minimum = min
max = _incidence.max
maximum = max
clamp = _incidence.clamp
compare = _incidence.compare
select = _incidence.select
table_lookup = _incidence.table_lookup
interpolated_table = table_lookup
roundtrip_expression = _incidence.roundtrip_expression


def retain_all():
    """Return a plain-data disposition that retains all evaluated stock."""
    return {
        "rule_ir_version": "v1",
        "numerical_semantics_version": "v1",
        "partition": {"kind": "retain_all"},
    }


def release_all(branch):
    """Return a plain-data disposition that releases all stock to ``branch``."""
    return {
        "rule_ir_version": "v1",
        "numerical_semantics_version": "v1",
        "partition": {"kind": "release_all", "branch": branch},
    }


def rule(compartment, substance, expression, disposition, parameters=()):
    """Return one plain-data rule document; the core validates it during compilation."""
    if isinstance(parameters, dict):
        parameters = [{"id": key, "value": value} for key, value in parameters.items()]
    else:
        parameters = list(parameters)
    return {
        "compartment": compartment,
        "substance": substance,
        "expression": expression,
        "disposition": disposition,
        "parameters": parameters,
    }


def model_document(**components):
    """Return a complete plain-data document mapping with protocol-version defaults."""
    document = {
        "document_version": "v1",
        "versions": {
            "rule_ir": "v1",
            "interpreter": "v1",
            "numerical_semantics": "v1",
            "canonical_encoding": "v1",
        },
    }
    document.update(components)
    return document


__all__ = [
    "CompiledModel", "CompletedRun", "PresenceSeries", "PresenceCountSeries", "compile_model", "literal", "param", "parameter", "input",
    "forcing", "projection", "add", "mul", "multiply", "power", "min", "minimum", "max",
    "maximum", "clamp", "compare", "select", "table_lookup", "interpolated_table",
    "roundtrip_expression", "retain_all", "release_all", "rule", "model_document",
]

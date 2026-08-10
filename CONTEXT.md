# Context

## Language

**Denotation line**:
A one-line statement, recorded in a module's `//!` doc before implementation, of what the module computes as a mathematical object (e.g. `snap : GeoCoord × FlowAccumulation → GridCoord`, pure). Carriers must be named domain types; if the line cannot be written, the design is not ready.
_Avoid_: equation, type signature, summary line

**Composition root**:
`src/main.rs`, the single place where config and environment are read, paths are resolved, `tracing` is initialized, raw input is parsed into domain types, and authority is granted. Library crates (`crates/*`) receive everything as arguments and never self-configure. Authority narrows at every call: pass the narrowest argument that suffices.
_Avoid_: setup code, wiring layer, boundary layer

**Domain type**:
A newtype, struct, or enum encoding a domain concept whose confusion or invalid state must fail to compile (IDs, coordinate systems, thresholds, lifecycle states). Constructed once at the composition root ("parse, don't validate"); no raw primitive crosses into `crates/*` where a domain type exists.
_Avoid_: wrapper class, validated primitive

**Isolation point**:
The single named place in a batch loop over independent items where a per-item failure may be caught, recorded with its cause, and skipped. Exists once per pipeline; every other error propagates with `?`.
_Avoid_: error swallowing, defensive catch

**Non-obviousness criterion**:
The admission test for a rule in AGENTS.md: a line belongs only if it is (a) an arbitrary project choice a model cannot infer, or (b) a practice that default LLM output violates. Practices a model already follows unprompted are excluded, as is anything rustfmt or clippy enforces mechanically.
_Avoid_: non-obvious rule, style guide entry

# ADR-0002: The Python binding is a thin boundary

## Status

Accepted

## Context

incidence was built as a finished conserved-flow engine that nothing outside Rust could reach. A model artifact could not be deserialised; the only way to construct one was `ModelArtifact::builder` fed by typed fallible Rust calls, and the only model anyone had ever built was a private test-target document invented by the fixture author precisely because no public whole-model form existed. ADR-0001 recorded the engine's contracts but deliberately deferred every question of surface.

Python is the engine's first real consumer, and binding it forced a question ADR-0001 left open: where does validation live, and how much does the boundary know about the domain?

Two positions were genuinely available.

The binding could offer Python-side ergonomics over a core-validated schema — helper objects that assemble a rule intermediate representation in Python, check what they can locally, and present a surface shaped for the people who actually write models. Hydrologists write these models, and a surface that names reaches, flows and demands is far easier to use than one that names compartments, substances and dispositions.

Or the binding could be a thin passthrough — plain data in, plain data out, with every judgement about whether a model is legal made once, in the core.

The choice is hard to reverse in both directions. A document format handed to callers is a permanent wire commitment. A validation implemented in Python is a second implementation of legality that can drift from the first, and drift in a conservation engine is not a cosmetic defect: it produces models that one layer believes are legal and the other does not.

## Decision

The binding is a thin boundary. It owns exactly three things: marshalling plain Python data across the FFI edge, containing panics so that no failure crosses as an interpreter-killing error, and constructing expression trees as serialisable data.

Every validation stays in the core. A model document is decoded and validated by exactly one implementation, so exactly one answer exists to whether a model is legal. The binding contains no schema knowledge it could disagree with.

The engine gained a public whole-model document format to make this possible, and the authoritative log's canonical bytes became public so that a caller can obtain and compare a run's log rather than being told about it.

The binding wears the engine's vocabulary — compartments, substances, transfers, dispositions — and does not name water. Expression combinators return serialisable document nodes, never Python callables, so a rule remains content-addressable data on both sides of the boundary. Results carry per-timestep presence, so an explicit zero, a position outside the modelled horizon, and a substance the run never declared remain three distinguishable answers at the boundary, exactly as they are inside the engine.

The whole model crosses once per run. There is no per-timestep callback into Python.

## Consequences

The Python surface is deliberately unfriendly to hydrologists. It is not the modelling API, and it should not grow into one. The friendly, water-naming layer belongs above this boundary, in the taqsim rebuild, where domain vocabulary is appropriate and where a second implementation of legality is not being created.

The model document format is now a permanent wire commitment. Changing it is a breaking change to every caller, and it is content-addressed, so a change to the format is a change to model identity.

The log's canonical bytes are public API. Callers may obtain a run's log and compare or replay it; the engine can no longer treat that byte sequence as an internal detail.

Because validation is single-sourced, an invalid model fails at the boundary with the core's own error text rather than a paraphrase. Error quality at the boundary is therefore the core's responsibility, not the binding's.

The rule language has been exercised only by the seven-rule hydrology fixture. Whether it is adequate for real hydrology remains open, and its resolution condition is a real consumer — the taqsim rebuild.

## Alternatives

### Python-side ergonomics over a core-validated schema

Rejected. It creates a second place where a model's legality is decided, and the two can drift. It also requires the boundary to name domain concepts in order to be friendlier, which breaks the engine's substance-neutrality: a surface that says "reach" and "demand" is no longer a substance-agnostic conserved-flow engine's surface. The ergonomic layer is genuinely wanted, but it belongs one level up, where naming water is correct.

### Persistence at the boundary

Deferred, not rejected. Writing a model or a log to disk and reading it back is a second permanent wire commitment, and it should be shaped by a real consumer's needs rather than guessed at here. Nothing is made harder by waiting: the log carries a completion seal and a digest, and the model artifact is content-addressed.

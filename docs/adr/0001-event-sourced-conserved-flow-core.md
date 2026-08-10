# ADR-0001: Event-sourced conserved-flow core

## Status

Accepted

## Context

Taqsim was described as event-sourced while its simulation state remained authoritative in mutable private fields. Its durable event list could be disabled without changing simulation results, while the events that drove a step travelled through a separate transient list and were discarded. A positive-value guard also omitted dry timesteps, making a zero indistinguishable from a missing record. Keeping an inert log beside mutable authoritative state therefore allowed the two representations to drift and left completeness unenforced.

Conserved extensive quantities provide a natural reconstruction law: stock is the initial stock folded with the signed transfers into and out of each compartment. If the transfer history is authoritative, replay tests whether that history is complete; if mutable state is independently authoritative, an audit log can become incomplete without affecting execution.

The core must remain substance- and unit-neutral. It must enforce conservation, complete disposition, deterministic ordering, and replay without interpreting what a substance or destination means. Rules decide transfers from typed current inputs, immutable parameters, and deterministic projections. Intelligence interprets results only after engine execution.

## Decision

The conserved-flow core uses one authoritative append-only event log. A run begins by binding its immutable, transitively content-addressed model artifact and initial stocks, records atomic transfers between typed endpoints, and may end with a completion proof. The model artifact includes every input that can affect execution, including forcing values and numerical-semantics versions, and historical artifacts remain retrievable. Changing any such input creates a different artifact identity; replay against a different identity is rejected rather than reconciled.

Running stock is maintained as the same deterministic fold used by replay, not as an independent source of truth. An unsealed valid prefix and its bound artifact contain everything required to reconstruct state and continue. Replay is the completeness mechanism: the authoritative facts must reconstruct every running stock bit-identically at every timestep.

The engine, rules, and intelligence remain separate. The engine owns conservation, exhaustive disposition, authoritative ordering, and replay. Rules own what happens within a step, including inter-substance coupling, but may depend only on typed current inputs, immutable model parameters, and deterministic typed projections of authoritative facts. Intelligence owns meaning and runs strictly after the engine.

Every rule disposition explicitly accounts for every modelled substance, including material retained at its source; no remainder is inferred from subtraction. Every authoritative record and reader contract preserves three distinct states: an explicit zero, a value or record that is not present, and a substance that was not modelled. Substances are conserved extensive quantities only. Amounts are extensive; rates and concentrations are projections. The engine emits no derived quantities or domain interpretation, and the terms “loss”, “spill”, “waste”, “excess”, “consumed”, and “deficit” are excluded from its vocabulary.

Deterministic numerical ordering, including floating-point summation order, is part of the event and replay contract. Projections, indexes, memoisation, compiled tables, rolling windows, and other caches are disposable: they may improve execution but must be exactly rebuildable from the bound artifact and authoritative log.

This decision specifies architecture contracts only. It does not choose record field layouts, Rust representations, encodings, expression-tree nodes, projection implementations, or execution mechanics; those belong to later decisions and implementation milestones.

## Consequences

- A completed or resumable run has one authority. State, history views, and caches are reconstructible from the artifact and log rather than reconciled with a second mutable truth.
- Immutable artifact binding makes runs reproducible and makes changed forcing or semantics an explicit incompatibility. This requires content addressing and durable historical artifact retention.
- Disposable projections and caches can be rebuilt exactly, but their update order and initial states become versioned deterministic contracts.
- Exhaustive dispositions and the distinction among zero, not present, and not modelled require explicit representations and rejection of incomplete inputs rather than defaults.
- Bit-identical replay constrains numerical algorithms and parallel execution to a specified deterministic order.
- Substance neutrality keeps interpretation and derived quantities out of the engine, requiring rule libraries and downstream intelligence to supply domain meaning.
- The event log may be larger and the implementation must maintain replay and continuation tests, but no independently authoritative mutable-state path is permitted.

## Alternatives

### Mutable authoritative state with an audit log

Rejected. This is the architecture observed in Taqsim: execution can remain correct according to mutable fields while logging silently omits facts. The log then cannot prove completeness, reconstruct state, or distinguish zero activity from absent history. Treating the log as optional audit output would preserve the failure this decision exists to prevent.

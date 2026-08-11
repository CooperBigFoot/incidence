# Project domain context

## Canonical terms

| Term | Meaning |
|---|---|
| Authoritative log | The ordered, append-only sequence of records that is the sole durable authority for a run's history. Reconstructible run state is folded from this log; it is not maintained as a separate authority. |
| Record | One immutable fact in the authoritative log. A record contributes to run history through its position and content; it is not a mutable state snapshot. |
| Genesis | The first record of an authoritative log. It identifies the run, binds it to one model digest and numerical-semantics version, and supplies the initial stocks from which later state is folded. |
| Transfer | An atomic movement of one or more registered substances between two typed endpoints at one timestep. It records extensive amounts, not an interpretation or a derived quantity. |
| Run-completed seal | The optional terminal record that proves a run completed through its declared final timestep and binds the final transfer count and log digest. No record follows a valid seal. |
| Resumable prefix | A valid Genesis-led record sequence without a Run-completed seal. It is explicitly incomplete and may be continued using the model artifact bound by Genesis; it must never be reported as a completed run. |
| Model artifact | The immutable, historically retrievable, transitively content-addressed description of everything required to reproduce a run: topology, registry, initial stocks and projector states, timestep calendar and horizon, forcing values, rule parameters and expressions, units, canonical encoding, and semantic versions. |
| Model digest | The canonical content identity of a model artifact. Genesis binds a run to this identity, and any change to transitively covered content, including one forcing value, produces a different digest rather than a compatible variant. |
| Finite compartment | A typed endpoint that holds non-negative stock and cannot transfer more of a substance than it holds. It is structurally distinct from a boundary account. |
| Boundary account | A typed, signed external counterparty to transfers. Its structural type, rather than a boolean or naming convention, distinguishes it from a finite compartment. |
| Substance registry | The model artifact's closed declaration of the independently conserved extensive quantities modelled by a run. Registry membership determines whether a requested substance is modelled; it does not assign interpretation or units. |
| Stock | The extensive amount of each registered substance attributed to an endpoint at a timestep, obtained as a running fold from Genesis and subsequent transfers. Stock is authoritative only insofar as it is reconstructible from the authoritative log. |
| Disposition | A rule's explicit, exhaustive partition of every available registered substance among retention and outgoing transfers. Retention is authored, never inferred as a remainder, and the complete partition must equal the available stock. |
| Projection | A non-authoritative, deterministic fold of logged facts and a declared initial projector state that supplies a typed view such as a lag, aggregate, recurrence, rate, or concentration. Projection state is disposable and exactly rebuildable. |
| Replay | Deterministically folding an authoritative log from Genesis under its bound model artifact to reconstruct run state and verify compatibility and completeness. Replay never reconciles the log with a different artifact. |
| Zero | A present, applicable value for a modelled substance whose amount equals the additive identity. A timestep inside the declared horizon with no corresponding transfer may therefore have an explicit zero. |
| Absent | The state also described as "not present": no value exists at the requested location in the artifact-declared domain, such as a timestep outside the declared horizon. It makes no claim that the amount is zero. |
| Not modelled | The requested substance is outside the substance registry bound to the run. It is neither an absent value nor a zero value and must not be converted to either. |
| Denotation line | A one-line statement, recorded in a module's `//!` doc before implementation, of what the module computes as a mathematical object, for example `snap : GeoCoord × FlowAccumulation → GridCoord   (pure)`. Its carriers are named domain types; if the line cannot be written, the design is not ready. |
| Composition root | `src/main.rs`, the sole place where configuration and environment are read, paths are resolved, tracing is initialized, raw input is parsed into domain types, and I/O authority is granted. Library crates receive only the narrow arguments they need and never self-configure. |
| Domain type | A newtype, struct, or enum encoding a domain concept whose confusion or invalid state must fail to compile. Raw inputs are converted to domain types at the composition root, and no raw primitive crosses into a library crate where a domain type exists. |
| Isolation point | The single named place in a batch loop over independent items where one item's failure may be caught, recorded with its cause, and skipped. Every other error propagates. |
| Non-obviousness criterion | The admission test for a rule in `AGENTS.md`: include it only when it is an arbitrary project choice that cannot be inferred from the code, or a practice that default model output violates; omit mechanically enforced or already-default practice. |
| Rule expression | A versioned, serialisable, closed tree of typed neutral references, finite literals, ordered scalar operations, comparisons, and conditional selection. It is immutable data and carries no evaluator or private authority. |
| Expression value kind | The compile/construction-time distinction between a scalar-valued rule expression and a truth-valued predicate. Scalar and truth operands are not interchangeable. |
| Partition expression | One of five closed, substance- and unit-neutral partition shapes: retain-all, release-all, fixed-fraction split, exogenous series, or constant-fraction transfer. |
| Rule reference | A validated typed identity naming an opaque current input, immutable parameter, forcing series, deterministic projection, interpolation table, or transfer branch; resolution belongs to the model program and later runtime layers. |

## Aliases to avoid

| Avoid | Use instead | Why |
|---|---|---|
| Audit log | Authoritative log | An audit log sounds secondary to mutable state; this log is the run's sole durable authority. |
| Event | Record | `Record` names an immutable authoritative fact, while `event` alone does not state authority or persistence. |
| Initialization record | Genesis | Genesis is the required first record and binds the run to its reproducibility inputs, not merely an initialization action. |
| Completion flag | Run-completed seal | Completion is an explicit terminal record with evidence, not a boolean attached elsewhere. |
| Partial run | Resumable prefix | `Partial run` does not say whether the sequence is valid or can be continued; `resumable prefix` does. |
| Model configuration | Model artifact | Configuration understates the artifact's immutable forcing data, initial state, encoding, and semantic-version coverage. |
| Checksum | Model digest | The digest is the model artifact's content identity and run binding, not merely an accidental-corruption check. |
| Node | Finite compartment or boundary account | `Node` hides the structural distinction between a non-negative stock holder and a signed external counterparty. |
| Source or sink | Boundary account | Directional names can disguise an endpoint's role and rely on naming convention instead of its structural type. |
| Substance list | Substance registry | The registry is the closed authority for what the run models, not an incidental collection. |
| Cached state | Projection state | A projection is non-authoritative and rebuildable from declared initial projector state and logged facts; `cached state` does not express that contract. |
| Rule output | Disposition | A disposition is exhaustive over every available registered substance and includes explicit retention; a generic output need not be. |
| Reconstruction | Replay | Replay includes deterministic folding plus artifact compatibility and run-completeness semantics. |
| Missing | Absent or Not modelled | `Missing` conflates a value outside the artifact-declared domain with a substance outside the run's registry. |
| Equation, type signature, or summary line | Denotation line | These names do not require a module to state its mathematical object before implementation. |
| Setup code, wiring layer, or boundary layer | Composition root | These names obscure that one place owns parsing and authority and must narrow both before calling libraries. |
| Wrapper class or validated primitive | Domain type | A domain type makes confusion or invalid states unrepresentable; it is not validation decoration. |
| Error swallowing or defensive catch | Isolation point | The isolation point is one named, recorded exception for independent batch items, not general suppression. |
| Non-obvious rule or style guide entry | Non-obviousness criterion | The term names the admission test, not any individual rule or a general style catalogue. |

## Relationships

| Concepts | Relationship |
|---|---|
| Authoritative log, Record, Genesis, Transfer, Run-completed seal | An authoritative log begins with exactly one Genesis, continues with zero or more Transfers, may end with one Run-completed seal, and contains no record after that seal. |
| Genesis, Model digest, Model artifact | Genesis permanently binds the run to the model digest that identifies the one model artifact used for replay or continuation. |
| Genesis, Transfer, Stock | Genesis supplies initial stocks; folding Transfers in authoritative order yields every later stock. |
| Transfer, Finite compartment, Boundary account | Every Transfer names two typed endpoints, so movement remains closed-world while finite-compartment and boundary-account invariants stay distinct. |
| Substance registry, Stock, Transfer, Disposition | The registry defines the substances that stocks, transfers, and dispositions must cover; a substance outside it is Not modelled. |
| Disposition, Transfer | A validated disposition determines explicit retention and the outgoing Transfers that may be appended atomically. |
| Authoritative log, Projection, Replay | Projection and replay read authoritative records deterministically; projections are disposable views, while replay reconstructs run state and checks the log's binding and completion semantics. |
| Run-completed seal, Resumable prefix | Presence of a valid seal proves completion; its absence leaves a resumable prefix and must not be interpreted as completion. |
| Zero, Absent, Not modelled | These are mutually exclusive states: zero is a present value, absent means no value at the requested artifact location, and Not modelled means the requested substance is outside the run's registry. |
| Finite compartment, Boundary account | They are disjoint endpoint kinds expressed by structural types, never by a boolean flag or a name convention. |
| Composition root, Domain type | The composition root converts raw input into domain types once and passes only the narrow domain values required by library code. |
| Denotation line, Module | Every module states its mathematical object in a one-line `//!` denotation before implementation. |
| Rule expression, Rule reference, Projection | A rule expression may read a typed projection reference, while the projection specification and rebuildable projector state remain separate and are defined later. |
| Rule expression, Partition expression, Numerical-semantics version | Both IR trees carry the rule-IR and numerical-semantics versions; expression child order and fraction accumulation order are part of their canonical identity. |

## Ambiguities

| Topic | Current interpretation | Resolution condition |
|---|---|---|
| Sufficiency of the closed rule vocabulary | The vocabulary can construct references and combinator shapes needed by the planned sharp fixtures, but adequacy for a real hydrology rule set is not yet demonstrated. | Resolve only when the complete fixture rule set is built and executed against the public IR; inability to express a fixture requires a closed-vocabulary design decision, never an opaque extension. |

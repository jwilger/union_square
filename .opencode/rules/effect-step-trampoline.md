# Rule: Effect, Step, And Trampoline Orchestration

Non-hot-path workflows SHOULD use explicit effects and step/trampoline orchestration when sequencing side effects would otherwise be hidden in domain or application logic.

## Effects

Effects describe intended side effects. They MUST NOT perform IO themselves.

Effects MAY represent:

- Persisting audit facts.
- Loading or updating read models.
- Calling provider-independent adapters.
- Emitting telemetry requests for the shell to interpret.
- Scheduling retryable work.

## Steps

Steps represent pure workflow progress. A step MAY complete with a result or request an effect.

Step functions MUST be deterministic for the same input state and observations.

## Trampoline

The trampoline MUST belong to the imperative shell. It interprets effects, executes IO, feeds observations back into the workflow, and stops when the workflow completes or fails.

## Where To Use

Use this pattern for:

- Audit persistence coordination.
- Session analysis workflows.
- Test extraction workflows.
- Retryable non-hot-path orchestration.

This pattern MUST NOT be used inside measured hot-path forwarding or streaming loops unless benchmarks show the overhead is acceptable.

## Enforcement

- Architecture review against `docs/architecture/ARCHITECTURE.md`.
- Tests for pure step transitions and trampoline effect interpretation.
- Code review by `functional-architecture-expert` and `async-rust-expert`.

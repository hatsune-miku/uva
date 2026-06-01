# uva (uv automations) — Design Spec

**Date:** 2026-06-01
**Status:** Approved (ready for implementation)

## Summary

`uva` is a thin Rust CLI wrapper around [`uv`](https://docs.astral.sh/uv/) that gives
Python a `yarn`-like experience. It abstracts away the concept of virtual
environments entirely: users never think about `venv`, yet still get the
isolation `venv` provides. `uva` always defers to whatever Python version `uv`
currently has active — it never switches Python versions.

### Target users
- People who want one tool to handle the Python environment on their machine.
- People who find conda/miniconda too heavy and protocol-complex.
- People who want `uv` but are confused by its distinct workflow.
- People who don't want to fuss over dependency versions and just want to run a
  script as fast as possible.

### Prerequisite
- `uv` must be installed on the system.

## Architecture

Two layers, deliberately separated so the decision logic is unit-testable
without spawning subprocesses:

1. **Decision layer (pure, no side effects):** given the CLI args and a view of
   the filesystem, produce a `Plan` value describing what to do.
2. **Executor (side effects):** verify `uv` is present, echo the command to
   stderr, spawn `uv` with inherited stdio, and propagate its exit code.

### Module layout (Rust binary named `uva`)
- `main.rs` — entry point; parse args → `Plan` → execute, map result to exit code.
- `cli.rs` — argument dispatch and usage text.
- `detect.rs` — dependency-file detection and `run` script resolution (pure).
- `plan.rs` — the `Plan` enum (the contract between decision layer and executor).
- `runner.rs` — `uv` presence check, command echo, subprocess spawn, exit-code
  propagation.

### The `Plan` enum (contract)
```
enum Plan {
    RunUv(Vec<String>),            // run `uv <args>`, pass exit code through
    RunUvSeq(Vec<Vec<String>>),    // run several `uv` commands in order; stop on first failure
    EnsureVenvThenRun(Vec<Vec<String>>), // like RunUvSeq but the first `uv venv` runs only if `.venv` is absent
    PrintUrl,                      // print the uv install URL to stdout, exit 0
    Usage,                         // print usage to stderr, exit 2
    Fail(String),                  // print message to stderr, exit 1
}
```
(Exact representation may be refined during implementation; the principle —
decisions are data, execution is separate — is the requirement.)

## CLI dispatch

`uva [args...]`:
- **No args** → behave as `uva install`.
- **First arg is a known keyword** (`install`, `run`, `start`, `how-to-install-uv`)
  → dispatch to that subcommand. Keywords win over same-named files.
- **`--help` / `-h`** → print usage to stdout, exit 0.
- **`--version` / `-V`** → print version, exit 0.
- **Otherwise** treat the first arg as a filename:
  - if the file exists → behave as `uva run <file> [remaining args...]`
  - else → print usage to stderr, exit 2.

## Commands

### `uva install` (and bare `uva`)
Detect the project's dependency source, in priority order, and run the matching
`uv` command:
1. `uv.lock` present → `uv sync`
2. `pyproject.toml` present → `uv sync`
3. `requirements.txt` present → ensure a project-local environment, then install:
   - run `uv venv` **only if** `.venv` does not already exist,
   - then `uv pip install -r requirements.txt`.
4. none of the above → error: `当前目录不是一个 Python 项目` (exit 1).

`uv sync` and `uv run` create/manage `.venv` implicitly; the requirements.txt
path creates it explicitly via `uv venv`. The user never sees venv mechanics.

### `uva run [filename] [extra args...]`
Generate and run the `uv` command that runs the script: `uv run <file> [extra args...]`.

`filename` need not end in `.py`.

When `filename` is omitted, resolve the default script in priority order
(first match wins):
1. the **single** `*.py` file in the current directory (only if exactly one exists)
2. the **single** `*.py` file in `src/` (only if exactly one exists)
3. `main.py`
4. `src/main.py`
5. else → error: `未指定源文件。已尝试过：…` listing what was tried (exit 1).

Trailing args after the filename are forwarded to the script.

### `uva start [filename]`
Identical to `uva run [filename]`.

### `uva <filename>`
Covered by dispatch: if the arg is an existing file, equivalent to
`uva run <filename>`; otherwise usage.

### `uva how-to-install-uv`
Print exactly `https://docs.astral.sh/uv/getting-started/installation/` to
stdout and exit 0. This command does **not** require `uv` to be installed.

## Cross-cutting behavior

- **uv pre-check:** before executing any `uv` command, verify `uv` resolves on
  PATH. If missing, print an error that points at the install URL and exit
  non-zero. `how-to-install-uv`, `--help`, and `--version` skip the check.
- **Command echo:** before executing, print `$ uv <args...>` to **stderr** so
  users learn the underlying `uv` command without polluting program stdout.
- **Exit codes:** usage/argument errors → 2; "not a Python project" / "no
  source file" → 1; otherwise the wrapped `uv` command's exit code is passed
  through unchanged.
- **Python version:** never changed. `uva` uses whatever Python `uv` has active.
  If the user wants a different version, they use `uv` directly.

## Testing

- **Unit tests (`detect.rs`):** every branch of dependency detection and script
  resolution, using temporary directories (`tempfile` dev-dependency). No `uv`
  required.
- **Unit tests (`cli.rs`):** arg vectors → expected `Plan`, covering keyword
  dispatch, the no-arg case, the filename-fallback case, and usage.
- **Smoke tests (end-to-end):** run the built binary in a temp project
  (`uv` is available in the dev environment), e.g. `uva how-to-install-uv` and
  `uva install` against a minimal `pyproject.toml`.

## Out of scope
- Switching Python versions.
- Named/multiple virtual environments.
- Packaging/publishing `uva` itself (just needs `cargo build` to succeed).
- Any dependency-version management beyond what the chosen `uv` command does.

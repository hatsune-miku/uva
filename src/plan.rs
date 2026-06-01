//! `Plan` is the contract between the pure decision layer (`cli`/`detect`)
//! and the side-effecting executor (`runner`). Decisions are data.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Run each step in order, stopping at the first failure.
    Steps(Vec<Step>),
    /// Print the uv install URL to stdout, exit 0.
    PrintUrl,
    /// Print usage to stdout, exit 0.
    Help,
    /// Print version to stdout, exit 0.
    Version,
    /// Print usage to stderr, exit 2.
    Usage,
    /// Print an error message to stderr, exit 1.
    Fail(String),
}

impl Plan {
    /// Convenience: a plan that is just a sequence of `uv` commands.
    pub fn uv(cmds: Vec<UvCmd>) -> Plan {
        Plan::Steps(cmds.into_iter().map(Step::Uv).collect())
    }
}

/// One step of a `Plan`. Steps run in order; the first failure aborts the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Invoke `uv` with the given command.
    Uv(UvCmd),
    /// Append these package specs to `requirements.txt` (creating it if
    /// needed), skipping any already present by normalized name.
    AppendRequirements(Vec<String>),
    /// Remove any `requirements.txt` lines matching these package names.
    RemoveRequirements(Vec<String>),
    /// Set the global `uv.toml` index to the Tsinghua mirror.
    SetGlobalIndex,
    /// Remove `[[index]]` sections from the global `uv.toml`.
    ClearGlobalIndex,
}

/// Whether a `uv` command should run, depending on whether `.venv` exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenvGate {
    /// Run unconditionally.
    Always,
    /// Run only when `.venv` is absent (used for `uv venv`).
    OnlyIfMissing,
    /// Run only when `.venv` exists (best-effort, e.g. uninstall).
    OnlyIfPresent,
}

/// A single `uv` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UvCmd {
    /// Arguments passed to `uv` (excluding the program name).
    pub args: Vec<String>,
    /// Condition on `.venv` existence under which this command runs.
    pub gate: VenvGate,
}

impl UvCmd {
    pub fn new<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        UvCmd {
            args: args.into_iter().map(Into::into).collect(),
            gate: VenvGate::Always,
        }
    }

    /// Run this command only if `.venv` does not already exist.
    pub fn only_if_venv_missing(mut self) -> Self {
        self.gate = VenvGate::OnlyIfMissing;
        self
    }

    /// Run this command only if `.venv` already exists.
    pub fn only_if_venv_present(mut self) -> Self {
        self.gate = VenvGate::OnlyIfPresent;
        self
    }
}

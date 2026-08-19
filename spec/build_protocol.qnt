// Lattice Build Protocol (v0 stub).
// Quint will own allowed state transitions. This file is a named placeholder
// so we do not invent a second lifecycle in Rust.

module build_protocol {
  type Phase = | Dirty | Compiled | NeedsResolve | Resolved | Planned | Rendered

  type State = { phase: Phase }

  action compile = all {
    state.phase == Dirty,
    state' = { phase: Compiled }
  }
}

// Lattice Build Protocol (v0 stub).
// Quint will own allowed state transitions. This file is a named placeholder
// so we do not invent a second lifecycle in Rust.

module build_protocol {
  type Phase = | Dirty | Compiled | NeedsResolve | Resolved | Planned | Rendered

  var phase: Phase

  action compile = all {
    phase == Dirty,
    phase' = Compiled,
  }

  action resolve = all {
    phase == Compiled or phase == NeedsResolve,
    phase' = Resolved,
  }

  action plan = all {
    phase == Resolved or phase == Compiled,
    phase' = Planned,
  }

  action render = all {
    phase == Planned or phase == Resolved,
    phase' = Rendered,
  }

  val compileBeforeResolve = true
}

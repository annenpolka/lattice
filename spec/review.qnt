// Propose → Apply / Reject. Current VEL does not change until Apply.
// Rust: Engine::propose / apply_proposal / reject_proposal.
module review {
  type ReviewState = | Idle | Proposed | Applied | Rejected

  var state: ReviewState
  var velUnchanged: bool

  action init = all {
    state' = Idle,
    velUnchanged' = true,
  }

  action stutter = all {
    state' = state,
    velUnchanged' = velUnchanged,
  }

  action propose = all {
    state == Idle,
    state' = Proposed,
    velUnchanged' = true,
  }

  action apply = all {
    state == Proposed,
    state' = Applied,
    velUnchanged' = false,
  }

  action reject = all {
    state == Proposed,
    state' = Rejected,
    velUnchanged' = true,
  }

  action step = any {
    propose,
    apply,
    reject,
    stutter,
  }

  val invariantBeforeApply = (state == Proposed or state == Idle) implies velUnchanged
  val invariantRejectKeepsSource = (state == Rejected) implies velUnchanged
  val invariantApplyMutates = (state == Applied) implies not(velUnchanged)
  val inv = all {
    invariantBeforeApply,
    invariantRejectKeepsSource,
    invariantApplyMutates,
  }

  run rejectKeepsSourceTest =
    init.then(propose).then(reject).then(all {
      assert(state == Rejected),
      assert(velUnchanged),
      stutter,
    })

  run applyMutatesSourceTest =
    init.then(propose).then(apply).then(all {
      assert(state == Applied),
      assert(not(velUnchanged)),
      stutter,
    })
}

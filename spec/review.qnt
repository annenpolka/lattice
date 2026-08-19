// Propose → Apply / Reject. Current VEL does not change until Apply.
module review {
    type ReviewState = | Idle | Proposed | Applied | Rejected

    var state: ReviewState
    var velUnchanged: bool

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

    val invariantBeforeApply = (state == Proposed or state == Idle) implies velUnchanged
    val invariantRejectKeepsSource = (state == Rejected) implies velUnchanged
    val invariantApplyMutates = (state == Applied) implies not velUnchanged
}

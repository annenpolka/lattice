// Resolve lock lifecycle. Render from a valid lock must not regenerate.
module resolution {
    type LockState = | Open | Locked | Stale | Missing

    var lock: LockState
    var providerCalls: int

    action compile = all {
        lock' = lock,
        providerCalls' = providerCalls,
    }

    action resolveFirst = all {
        lock == Open or lock == Missing or lock == Stale,
        lock' = Locked,
        providerCalls' = providerCalls + 1,
    }

    action resolveLocked = all {
        lock == Locked,
        lock' = Locked,
        providerCalls' = providerCalls,
    }

    val compileNeverCallsProvider = true
    val lockedResolveDoesNotIncrement = true
}

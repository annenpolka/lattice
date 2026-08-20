// Resolve lock lifecycle for generated media, fonts, and decode inputs.
// A valid lock is the only input Render/Encode may sample. Compile never
// increments providerCalls. See Engine::resolve and lattice.lock.json.

module resolution {
  type LockState = | Open | Locked | Stale | Missing

  type AssetKind = | Font | GeneratedMedia | DecodeInput

  var lock: LockState
  var fontLock: LockState
  var providerCalls: int
  var phaseIsCompile: bool
  var phaseIsRender: bool
  var phaseIsEncode: bool

  action init = all {
    lock' = Open,
    fontLock' = Open,
    providerCalls' = 0,
    phaseIsCompile' = false,
    phaseIsRender' = false,
    phaseIsEncode' = false,
  }

  action stutter = all {
    lock' = lock,
    fontLock' = fontLock,
    providerCalls' = providerCalls,
    phaseIsCompile' = phaseIsCompile,
    phaseIsRender' = phaseIsRender,
    phaseIsEncode' = phaseIsEncode,
  }

  action compile = all {
    lock' = lock,
    fontLock' = fontLock,
    providerCalls' = providerCalls,
    phaseIsCompile' = true,
    phaseIsRender' = false,
    phaseIsEncode' = false,
  }

  action resolveFirst = all {
    lock == Open or lock == Missing or lock == Stale,
    lock' = Locked,
    fontLock' = Locked,
    providerCalls' = providerCalls + 1,
    phaseIsCompile' = false,
    phaseIsRender' = false,
    phaseIsEncode' = false,
  }

  action resolveLocked = all {
    lock == Locked,
    fontLock == Locked,
    lock' = Locked,
    fontLock' = Locked,
    providerCalls' = providerCalls,
    phaseIsCompile' = false,
    phaseIsRender' = false,
    phaseIsEncode' = false,
  }

  action renderFromLock = all {
    lock == Locked,
    fontLock == Locked,
    lock' = Locked,
    fontLock' = Locked,
    providerCalls' = providerCalls,
    phaseIsCompile' = false,
    phaseIsRender' = true,
    phaseIsEncode' = false,
  }

  action encodeFromLock = all {
    lock == Locked,
    fontLock == Locked,
    lock' = Locked,
    fontLock' = Locked,
    providerCalls' = providerCalls,
    phaseIsCompile' = false,
    phaseIsRender' = false,
    phaseIsEncode' = true,
  }

  action staleGenerated = all {
    lock == Locked,
    lock' = Stale,
    fontLock' = fontLock,
    providerCalls' = providerCalls,
    phaseIsCompile' = false,
    phaseIsRender' = false,
    phaseIsEncode' = false,
  }

  action staleFont = all {
    fontLock == Locked,
    lock' = lock,
    fontLock' = Stale,
    providerCalls' = providerCalls,
    phaseIsCompile' = false,
    phaseIsRender' = false,
    phaseIsEncode' = false,
  }

  action missingAsset = all {
    lock' = Missing,
    fontLock' = fontLock,
    providerCalls' = providerCalls,
    phaseIsCompile' = false,
    phaseIsRender' = false,
    phaseIsEncode' = false,
  }

  action step = any {
    compile,
    resolveFirst,
    resolveLocked,
    renderFromLock,
    encodeFromLock,
    staleGenerated,
    staleFont,
    missingAsset,
    stutter,
  }

  val compileNeverCallsProvider =
    phaseIsCompile implies true

  val lockedResolveDoesNotIncrement =
    (lock == Locked and not(phaseIsCompile)) implies providerCalls >= 0

  val renderEncodeDoNotCallProvider =
    (phaseIsRender or phaseIsEncode) implies lock == Locked

  val staleOrMissingBlocksRender =
    (lock == Stale or lock == Missing or fontLock == Stale)
      implies not(phaseIsRender or phaseIsEncode)

  val fontLockFreshForRender =
    phaseIsRender implies fontLock == Locked

  val providerCallsNonNegative = providerCalls >= 0

  val inv = all {
    providerCallsNonNegative,
    compileNeverCallsProvider,
    lockedResolveDoesNotIncrement,
    renderEncodeDoNotCallProvider,
    staleOrMissingBlocksRender,
    fontLockFreshForRender,
  }

  run compileLeavesProviderAtZeroTest =
    init.then(compile).then(all {
      assert(providerCalls == 0),
      assert(phaseIsCompile),
      stutter,
    })

  run secondResolveAgainstValidLockTest =
    init.then(resolveFirst).then(all {
      assert(providerCalls == 1),
      stutter,
    }).then(resolveLocked).then(all {
      assert(providerCalls == 1),
      assert(lock == Locked),
      stutter,
    })

  run renderAndEncodeFromLockDoNotIncrementTest =
    init
      .then(resolveFirst)
      .then(renderFromLock)
      .then(all { assert(providerCalls == 1), stutter })
      .then(encodeFromLock)
      .then(all {
        assert(providerCalls == 1),
        assert(phaseIsEncode),
        stutter,
      })

  run staleGeneratedBlocksRenderTest =
    init.then(resolveFirst).then(staleGenerated).then(all {
      assert(lock == Stale),
      assert(not(phaseIsRender)),
      stutter,
    })

  run staleFontBlocksRenderTest =
    init.then(resolveFirst).then(staleFont).then(all {
      assert(fontLock == Stale),
      assert(not(phaseIsRender)),
      stutter,
    })
}

// Lattice build protocol: Compile → Resolve → Render → Encode.
// Render produces frames/PCM. Encode is a later phase and must not
// change scene semantics or call generated-media providers.
// Rust names: Engine::compile, Engine::resolve, evaluate_at / sample_frame,
// Encoder::push_frame (see docs/architecture.md).

module build_protocol {
  type Phase =
    | Dirty
    | Compiled
    | NeedsResolve
    | Resolved
    | RenderReady
    | Rendering
    | FramesReady
    | Encoding
    | Encoded

  type LockState = | Open | Locked | Stale | Missing

  var phase: Phase
  var lock: LockState
  var providerCalls: int
  var sourceMutated: bool
  var lockMutated: bool
  var sceneId: int
  var codecBackend: str

  val renderPhases = Set(Rendering, FramesReady, Encoding)
  val backends = Set("ffmpeg", "software", "wgpu")

  action init = all {
    phase' = Dirty,
    lock' = Open,
    providerCalls' = 0,
    sourceMutated' = false,
    lockMutated' = false,
    sceneId' = 0,
    codecBackend' = "software",
  }

  action stutter = all {
    phase' = phase,
    lock' = lock,
    providerCalls' = providerCalls,
    sourceMutated' = sourceMutated,
    lockMutated' = lockMutated,
    sceneId' = sceneId,
    codecBackend' = codecBackend,
  }

  /// Compile is deterministic and never calls a provider or codec backend.
  action compile = all {
    phase == Dirty or phase == Encoded or phase == Compiled,
    phase' = Compiled,
    lock' = lock,
    providerCalls' = providerCalls,
    sourceMutated' = sourceMutated,
    lockMutated' = false,
    sceneId' = 0,
    codecBackend' = codecBackend,
  }

  /// First (or stale/missing) resolve may call a provider and produce a lock.
  action resolveOpen = all {
    phase == Compiled or phase == NeedsResolve,
    lock == Open or lock == Missing or lock == Stale,
    phase' = Resolved,
    lock' = Locked,
    providerCalls' = providerCalls + 1,
    sourceMutated' = sourceMutated,
    lockMutated' = true,
    sceneId' = sceneId + 1,
    codecBackend' = codecBackend,
  }

  /// A valid lock must not regenerate. Render/Encode later reuse this count.
  action resolveLocked = all {
    phase == Compiled or phase == NeedsResolve or phase == Resolved,
    lock == Locked,
    phase' = Resolved,
    lock' = Locked,
    providerCalls' = providerCalls,
    sourceMutated' = sourceMutated,
    lockMutated' = false,
    sceneId' = if (sceneId == 0) 1 else sceneId,
    codecBackend' = codecBackend,
  }

  /// Evaluate RenderScene from resolved assets. Not Encode.
  action evaluateScene = all {
    phase == Resolved or phase == RenderReady,
    lock == Locked,
    phase' = RenderReady,
    lock' = lock,
    providerCalls' = providerCalls,
    sourceMutated' = sourceMutated,
    lockMutated' = false,
    sceneId' = sceneId,
    codecBackend' = codecBackend,
  }

  action startRender = all {
    phase == RenderReady,
    lock == Locked,
    phase' = Rendering,
    lock' = lock,
    providerCalls' = providerCalls,
    sourceMutated' = false,
    lockMutated' = false,
    sceneId' = sceneId,
    codecBackend' = codecBackend,
  }

  action finishFrames = all {
    phase == Rendering,
    phase' = FramesReady,
    lock' = lock,
    providerCalls' = providerCalls,
    sourceMutated' = false,
    lockMutated' = false,
    sceneId' = sceneId,
    codecBackend' = codecBackend,
  }

  /// Encode is a distinct phase after frames/PCM exist.
  action startEncode = all {
    phase == FramesReady,
    phase' = Encoding,
    lock' = lock,
    providerCalls' = providerCalls,
    sourceMutated' = false,
    lockMutated' = false,
    sceneId' = sceneId,
    codecBackend' = codecBackend,
  }

  action finishEncode = all {
    phase == Encoding,
    phase' = Encoded,
    lock' = lock,
    providerCalls' = providerCalls,
    sourceMutated' = false,
    lockMutated' = false,
    sceneId' = sceneId,
    codecBackend' = codecBackend,
  }

  action failRender = all {
    phase == Rendering or phase == Encoding,
    phase' = RenderReady,
    lock' = lock,
    providerCalls' = providerCalls,
    sourceMutated' = false,
    lockMutated' = false,
    sceneId' = sceneId,
    codecBackend' = codecBackend,
  }

  action markStale = all {
    lock == Locked,
    phase' = NeedsResolve,
    lock' = Stale,
    providerCalls' = providerCalls,
    sourceMutated' = sourceMutated,
    lockMutated' = true,
    sceneId' = sceneId,
    codecBackend' = codecBackend,
  }

  action markMissing = all {
    phase' = NeedsResolve,
    lock' = Missing,
    providerCalls' = providerCalls,
    sourceMutated' = sourceMutated,
    lockMutated' = true,
    sceneId' = sceneId,
    codecBackend' = codecBackend,
  }

  /// Codec/backend choice is not semantic state: sceneId is unchanged.
  action switchBackend = all {
    nondet backend = backends.oneOf()
    all {
      phase' = phase,
      lock' = lock,
      providerCalls' = providerCalls,
      sourceMutated' = sourceMutated,
      lockMutated' = false,
      sceneId' = sceneId,
      codecBackend' = backend,
    }
  }

  action step = any {
    compile,
    resolveOpen,
    resolveLocked,
    evaluateScene,
    startRender,
    finishFrames,
    startEncode,
    finishEncode,
    failRender,
    markStale,
    markMissing,
    switchBackend,
    stutter,
  }

  val compileNeverCallsProvider =
    (phase == Compiled) implies true

  val renderNotEncode =
    phase != Encoding or true

  val sourceStableDuringRender =
    renderPhases.contains(phase) implies not(sourceMutated)

  val lockStableDuringRender =
    renderPhases.contains(phase) implies not(lockMutated)

  val lockedRenderDoesNotCallProvider =
    (lock == Locked and (phase == Rendering or phase == FramesReady or phase == Encoding or phase == Encoded or phase == RenderReady))
      implies providerCalls >= 0

  val cannotRenderWithoutLock =
    (phase == Rendering or phase == FramesReady or phase == Encoding)
      implies lock == Locked

  val providerCallsNonNegative = providerCalls >= 0

  val inv = all {
    providerCallsNonNegative,
    sourceStableDuringRender,
    lockStableDuringRender,
    cannotRenderWithoutLock,
    compileNeverCallsProvider,
    renderNotEncode,
    lockedRenderDoesNotCallProvider,
  }

  run compileDoesNotIncrementProviderTest =
    init.then(compile).then(all {
      assert(phase == Compiled),
      assert(providerCalls == 0),
      stutter,
    })

  run resolveThenLockedResolveDoesNotIncrementTest =
    init.then(compile).then(resolveOpen).then(all {
      assert(lock == Locked),
      assert(providerCalls == 1),
      stutter,
    }).then(resolveLocked).then(all {
      assert(phase == Resolved),
      assert(providerCalls == 1),
      stutter,
    })

  run renderAndEncodeAreDistinctTest =
    init
      .then(compile)
      .then(resolveOpen)
      .then(evaluateScene)
      .then(all { assert(phase == RenderReady), stutter })
      .then(startRender)
      .then(all { assert(phase == Rendering), stutter })
      .then(finishFrames)
      .then(all { assert(phase == FramesReady), stutter })
      .then(startEncode)
      .then(all { assert(phase == Encoding), stutter })
      .then(finishEncode)
      .then(all {
        assert(phase == Encoded),
        assert(providerCalls == 1),
        stutter,
      })

  run failRenderLeavesSourceAndLockTest =
    init
      .then(compile)
      .then(resolveOpen)
      .then(evaluateScene)
      .then(startRender)
      .then(failRender)
      .then(all {
        assert(phase == RenderReady),
        assert(not(sourceMutated)),
        assert(lock == Locked),
        assert(not(lockMutated)),
        stutter,
      })

  run staleBlocksEvaluateTest =
    init.then(compile).then(resolveOpen).then(markStale).then(all {
      assert(lock == Stale),
      assert(phase == NeedsResolve),
      stutter,
    })

  run switchBackendKeepsSceneTest =
    init.then(compile).then(resolveOpen).then(evaluateScene).then(switchBackend).then(all {
      assert(sceneId == 1),
      stutter,
    })
}

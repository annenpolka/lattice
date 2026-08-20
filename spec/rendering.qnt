// Render ≠ Encode. Preview and export share resolved scene semantics.
// Codec backend selection is not Core/scene state.
// Rust: evaluate_at, sample_frame, CpuCompositor / GpuCompositor, Encoder.

module rendering {
  type LockState = | Open | Locked | Stale | Missing
  type RenderPhase =
    | Idle
    | SceneReady
    | Drawing
    | FramesReady
    | Encoding
    | Done
    | Failed

  var lock: LockState
  var phase: RenderPhase
  var previewScene: int
  var exportScene: int
  var semanticScene: int
  var codecBackend: str
  var sourceMutated: bool
  var lockMutated: bool
  var providerCalls: int

  val backends = Set("ffmpeg", "software", "wgpu")

  action init = all {
    lock' = Locked,
    phase' = Idle,
    previewScene' = 0,
    exportScene' = 0,
    semanticScene' = 1,
    codecBackend' = "software",
    sourceMutated' = false,
    lockMutated' = false,
    providerCalls' = 1,
  }

  action stutter = all {
    lock' = lock,
    phase' = phase,
    previewScene' = previewScene,
    exportScene' = exportScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = sourceMutated,
    lockMutated' = lockMutated,
    providerCalls' = providerCalls,
  }

  action evaluatePreview = all {
    lock == Locked,
    phase == Idle or phase == SceneReady or phase == Done,
    lock' = lock,
    phase' = SceneReady,
    previewScene' = semanticScene,
    exportScene' = exportScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = sourceMutated,
    lockMutated' = false,
    providerCalls' = providerCalls,
  }

  action evaluateExport = all {
    lock == Locked,
    phase == Idle or phase == SceneReady or phase == Done,
    lock' = lock,
    phase' = SceneReady,
    previewScene' = previewScene,
    exportScene' = semanticScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = sourceMutated,
    lockMutated' = false,
    providerCalls' = providerCalls,
  }

  action startDraw = all {
    lock == Locked,
    phase == SceneReady,
    lock' = lock,
    phase' = Drawing,
    previewScene' = previewScene,
    exportScene' = exportScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = false,
    lockMutated' = false,
    providerCalls' = providerCalls,
  }

  action finishDraw = all {
    phase == Drawing,
    lock' = lock,
    phase' = FramesReady,
    previewScene' = previewScene,
    exportScene' = exportScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = false,
    lockMutated' = false,
    providerCalls' = providerCalls,
  }

  /// Encode consumes already-drawn frames and mixed PCM.
  action startEncode = all {
    phase == FramesReady,
    lock' = lock,
    phase' = Encoding,
    previewScene' = previewScene,
    exportScene' = exportScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = false,
    lockMutated' = false,
    providerCalls' = providerCalls,
  }

  action finishEncode = all {
    phase == Encoding,
    lock' = lock,
    phase' = Done,
    previewScene' = previewScene,
    exportScene' = exportScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = false,
    lockMutated' = false,
    providerCalls' = providerCalls,
  }

  action rendererFail = all {
    phase == Drawing or phase == Encoding,
    lock' = lock,
    phase' = Failed,
    previewScene' = previewScene,
    exportScene' = exportScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = false,
    lockMutated' = false,
    providerCalls' = providerCalls,
  }

  action markStale = all {
    lock' = Stale,
    phase' = Idle,
    previewScene' = previewScene,
    exportScene' = exportScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = sourceMutated,
    lockMutated' = true,
    providerCalls' = providerCalls,
  }

  action markMissing = all {
    lock' = Missing,
    phase' = Idle,
    previewScene' = previewScene,
    exportScene' = exportScene,
    semanticScene' = semanticScene,
    codecBackend' = codecBackend,
    sourceMutated' = sourceMutated,
    lockMutated' = true,
    providerCalls' = providerCalls,
  }

  action switchBackend = all {
    nondet backend = backends.oneOf()
    all {
      lock' = lock,
      phase' = phase,
      previewScene' = previewScene,
      exportScene' = exportScene,
      semanticScene' = semanticScene,
      codecBackend' = backend,
      sourceMutated' = sourceMutated,
      lockMutated' = false,
      providerCalls' = providerCalls,
    }
  }

  action step = any {
    evaluatePreview,
    evaluateExport,
    startDraw,
    finishDraw,
    startEncode,
    finishEncode,
    rendererFail,
    markStale,
    markMissing,
    switchBackend,
    stutter,
  }

  val previewExportSameScene =
    (previewScene == 0 or exportScene == 0 or previewScene == exportScene)

  val encodeDoesNotChangeScene =
    (phase == Encoding or phase == Done) implies semanticScene == 1

  val failureDoesNotMutateSourceOrLock =
    phase == Failed implies (not(sourceMutated) and not(lockMutated))

  val staleMissingCannotDraw =
    (lock == Stale or lock == Missing)
      implies (phase != Drawing and phase != FramesReady and phase != Encoding)

  val backendIsNotSemantic =
    semanticScene == 1 or semanticScene == 0

  val providerFrozenAfterResolve = providerCalls == 1

  val inv = all {
    previewExportSameScene,
    encodeDoesNotChangeScene,
    failureDoesNotMutateSourceOrLock,
    staleMissingCannotDraw,
    backendIsNotSemantic,
    providerFrozenAfterResolve,
  }

  run previewAndExportShareSceneTest =
    init
      .then(evaluatePreview)
      .then(evaluateExport)
      .then(all {
        assert(previewScene == exportScene),
        assert(previewScene == semanticScene),
        stutter,
      })

  run encodeLeavesSceneUntouchedTest =
    init
      .then(evaluateExport)
      .then(startDraw)
      .then(finishDraw)
      .then(startEncode)
      .then(finishEncode)
      .then(all {
        assert(phase == Done),
        assert(semanticScene == 1),
        assert(exportScene == 1),
        stutter,
      })

  run backendSwitchDoesNotChangeSceneTest =
    init.then(evaluatePreview).then(evaluateExport).then(switchBackend).then(all {
      assert(previewScene == 1),
      assert(exportScene == 1),
      assert(semanticScene == 1),
      stutter,
    })

  run failDoesNotMutateTest =
    init.then(evaluateExport).then(startDraw).then(rendererFail).then(all {
      assert(phase == Failed),
      assert(not(sourceMutated)),
      assert(not(lockMutated)),
      stutter,
    })

  run staleBlocksDrawTest =
    init.then(markStale).then(all {
      assert(lock == Stale),
      assert(phase == Idle),
      stutter,
    })
}

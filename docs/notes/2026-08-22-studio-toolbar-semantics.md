# Studio toolbar observation — domain semantics

Date: 2026-08-22  
Observed base: `85b589ec260554f851c214731e607c7727c7cae8`

This is a docs-only observation of the current top-of-window button row after
the verb-license spine. It describes the row that is present; it does not
propose a new toolbar or reopen the spine locks.

## Live observation

The Linux UI fixture opened the real GPUI window at 1400×840. The single
flex-wrapped action bar appeared as two visual lines:

1. `Open Video…`, `Set In`, `Set Out`, `Split at Playhead`,
   `Delete Selected Clip`, renderer status, audio status, `CPU`, `GPU DX12`,
   `Play`, `Pause`, `Seek`, `Scrub`
2. `Save`, `Undo`, `Redo`, `Resolve`, `Copy locus JSON`, `Gain -3 dB`, `Fade`,
   `Zoom In`, `Zoom Out`

The capture is the whole identified Studio client, not a reconstructed mockup:
[Studio toolbar at the observed base](https://github.com/annenpolka/lattice/blob/77894c052f8d5b63783df875a9de6880dad4c991/docs/artifacts/2026-08-22-studio-toolbar-semantics.png?raw=true).

At first paint, here was Title `demo:title:1` and the touched projection was
Timeline. Engine named `title`, `set-position`, and `resize-overlay` as legal.
Timeline routed `title`; the one utterance spoke that position and resize were
routed on Canvas. This state is useful because none of the toolbar's
source/scene edit routes is licensed for the current Title.

## The three things the row must not collapse

- **Legality** belongs to Engine and the one committed `LocusId`. It answers
  which `SemanticEdit` verb has an exact target, scope, and effect here.
- **Routing** belongs to the touched projection. Toolbar can commit `trim`,
  `set-gain`, and `set-fade` for Source; `split` and `delete` for Scene. That is
  not a second Toolbar legal set.
- **Utterance** describes the one here. It discloses the Engine legal set and
  says where a legal verb is routed when the current gesture cannot commit it.
  A toolbar label is not authority to acquire or substitute another target.

Consequently, the row has five canonical edit verbs even though it has six
edit buttons: `Set In` and `Set Out` are two parameterizations of `trim`.
`Split at Playhead`, `Delete Selected Clip`, `Gain -3 dB`, and `Fade` utter
`split`, `delete`, `set-gain`, and `set-fade` respectively.

## Button-by-button reading

| Visible control | Domain reading | Locus / license consequence |
|---|---|---|
| `Open Video…` | Project/session open, not a `SemanticEdit` verb | Opens another source-backed session; it does not route an edit against the old here. |
| `Set In`, `Set Out` | Toolbar routes `trim` with one bound changed at playhead source time | Legal only for Source here. Scene/Title here is refused and spoken; no source lookup becomes a hidden retarget. |
| `Split at Playhead` | Toolbar routes `split` at playhead source time | Legal only for Scene here. A related Scene displayed from Source is Navigate information, not permission to adopt it. |
| `Delete Selected Clip` | Toolbar routes canonical `delete` | Legal only for Scene here. “Selected” means the one shared here, not a toolbar-private clip selection. |
| Renderer status, audio status | Observable runtime state, not verbs | Neither participates in Engine legality or changes here. |
| `CPU`, `GPU DX12` | Explicit renderer request | Application policy, not `SemanticEdit`; required DX12 does not silently fall back. |
| `Play`, `Pause` | Transport commands | Change playback state, not here or the Engine legal set. |
| `Seek` | Top-bar transport command that seeks to zero | The seek-verb name is relevant only because this control is in the top bar. It does not point. |
| `Scrub` | Transport-position command at the current playhead | It must not call `point_from_timeline_time`; here remains unchanged. |
| `Save` | Persist current source text | Session persistence, not an Engine-licensed edit verb. |
| `Undo`, `Redo` | Volatile source-backed session history | Restore working source states and recompile; they do not introduce another persistent history or selection model. |
| `Resolve` | Explicit Resolve phase | Provider I/O and lock persistence stay outside Parse/Compile/Evaluate and outside the legal-edit vocabulary. |
| `Copy locus JSON` | Agent-context projection | Serializes the same shared locus; it does not manufacture a prompt-only or toolbar-local target. |
| `Gain -3 dB` | Toolbar routes `set-gain` with `db = -3` | Legal only for Source here; Scene here speaks `needs-source-binding` and keeps the Scene locus. |
| `Fade` | Toolbar routes `set-fade` with a 500 ms fade-in | Legal only for Source here; no scene/source promotion is allowed. |
| `Zoom In`, `Zoom Out` | Timeline viewport commands | Projection-local view state, not semantic legality and not pointing. |

## What the observed Title state means

With Title as here, the toolbar routing set for that locus kind is empty while
Engine legality is non-empty. That mismatch is not evidence that title edits
are absent: `title` is committed on Timeline/Inspector and placement edits are
committed on Canvas. Conversely, the presence of trim/split/delete/gain/fade
buttons does not make those verbs legal for Title. Invoking one must refuse,
speak the typed target mismatch, preserve `demo:title:1`, and leave source text
unchanged.

The row therefore remains one consumer of the shipped spine:

```text
one LocusId → Engine legal set → one utterance
                    ↑
          Toolbar is routing only
```

No toolbar action warrants `target_source_locus`, `target_scene_locus`, scene
promotion on video click, a cross-surface modal, a freeze row/Core Freeze, a
per-view selection, or GPUI knowledge in Core.

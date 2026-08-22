# Studio toolbar gen2 — Flash

Date: 2026-08-22
Status: Second-generation chrome and routing proposal (Flash). Docs-only design note; not an implementation plan and not picking a winner.

![Lattice Studio Flash Chrome](https://github.com/annenpolka/lattice/blob/main/docs/notes/assets/2026-08-22-studio-toolbar-gen2-flash-header.png?raw=true)

---

## 1. Context & The Shared Premise

### Unanimous DELETE
The legacy Studio header contained a fixed, always-visible bank of locus-taking `SemanticEdit` buttons:
`Set In`, `Set Out`, `Split at Playhead`, `Delete Selected Clip`, `Gain -3 dB`, `Fade`.

This bank was deleted unanimously under the verb-license spine because:
1. **Target Fallthrough & Ghost Context**: A global button lacks explicit target bindings when "here" is ambiguous or mismatched.
2. **Surface Contradiction**: Having buttons that act on Source bindings or Scenes sitting detached above the Canvas and Timeline breaks the direct manipulation contract (`Manipulate / Navigate / Review`).
3. **Engine-Legality Parity**: Actions must be derived directly from Engine-named legality (`legal_edits_for(locus)`) for the committed locus, rather than hardcoded into a permanent UI button shelf.

### Invariant Locks (Reopened: NONE)
This proposal strictly abides by all settled interaction invariants:
- **One Locus, One Engine Legal Set, One Utterance**: There is only one semantic "here" across all projections (Canvas, Timeline, VEL, Inspector, Review, and External Agent context).
- **Projection-Local Overlap**: When a coordinate point hits multiple overlapping candidates, selection resolution stays local to the touched projection (candidate cards). Pointing remains unresolved until a card is picked or dismissed. No cross-surface modal is opened.
- **Video Click Points Source Clip**: Clicking a video clip in the Timeline names the `source:clip` identity directly without silent promotion to its containing `scene`.
- **No Per-View Selection**: Focus, hover, and playhead movements are not selections.
- **No Silent Retarget**: A gesture or control never mutates an implicit first-match target.
- **Title Fields Only on Title**: Dedicated property fields appear in Inspector if and only if "here" is a Title locus.

---

## 2. Flash Chrome: Visual & Affordance-First Replacement

The **Flash Chrome** proposes a crisp, non-verb application strip at the top of Studio. It strips all locus-dependent editing verbs and restricts top-level chrome exclusively to **global session lifecycle, project identity, transport, and device health status**.

```text
+-------------------------------------------------------------------------------------------------------------+
| [◆] Lattice Studio  |  gameplay-commentary/main.vel (•)  |   [⏮] [ ▶ Play ] [⏸]  00:01.240 / 00:08.500      |
|                                                          |   Renderer: CPU [DX12] | Audio: OK | [Save] [Resolve]|
+-------------------------------------------------------------------------------------------------------------+
|  [Sequence Tree]   |               [ Canvas / Preview ]              |            [ Inspector ]             |
|  - main.vel        |  +--------------------------------------------+ | Locus: title Hello (main.vel:16)     |
|    - scene demo    |  |               [ title Hello ]              | | Origin: invocation `title`           |
|      - clip fight  |  |                                            | |                                      |
|      - title Hello |  +--------------------------------------------+ | [ Utterance Disclosure Bar ]         |
|                    |                                                 | "Legal: title, set-position, resize" |
+--------------------+-------------------------------------------------+--------------------------------------+
|  [ Timeline ]                                                        | Zoom: [ - | + ]                      |
|  Track 0 [Video]   | [==== clip:fight (Source) ===================]  |                                      |
|  Track 1 [Title]   |        [==== title:Hello (Title) ====]          |                                      |
|  Track 2 [Audio]   | [~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~]  | Playhead: 00:01.240                  |
+-------------------------------------------------------------------------------------------------------------+
```

### Top Strip Membership Breakdown

| Cluster | Components | Why It Belongs in Top Strip |
|---|---|---|
| **Identity & File Lifecycle** | Brand Mark `[◆]`, Project Name, File Label (`main.vel`), Dirty indicator (`•`), `Open Video…`, `Save` (`Ctrl+S` / `Cmd+S`) | Global document lifecycle; independent of locus. |
| **A/V Transport Clock** | `Seek Start` (`⏮`), `Play` / `Pause` (`Space`), Current Timecode / Total Duration (`00:01.240 / 00:08.500`) | Transport and playhead clock are global session state, not semantic loci. |
| **Engine Compilation & Resolution** | `Resolve` (`Ctrl+R`), `Undo` / `Redo` | Triggers provider I/O resolution (`lattice.lock.json`) and session undo history. |
| **Runtime & Device Telemetry** | Renderer toggle (`CPU` / `GPU DX12` status pills), Audio Monitor status (`48kHz OK` / `Muted` / `Device Err`), `Copy locus JSON` | Exposes engine diagnostic and device health visibly without modal inspection. |

---

## 3. Explicit Routing for the Evicted Verbs

With the locus-taking verb bank deleted from the top toolbar, every single evicted verb is mapped to an explicit, discoverable route:

```text
               +-------------------------------------------------------------+
               |                       Committed Locus                       |
               +-------------------------------------------------------------+
                                      |                     |
                   [ LocusKind::Source ]                   [ LocusKind::Scene ]
                          |                                         |
            +-------------+-------------+             +-------------+-------------+
            |                           |             |                           |
       (trim)                   (gain & fade)       (split)                   (delete)
            |                           |             |                           |
            v                           v             v                           v
     Timeline Rail             Audio Inspector        Razor Tool / Shortcut     Context Menu /
     Direct Edge Drag          Parameter Sliders      or Timeline Context Menu  Delete Key (`Backspace`)
```

### 1. `trim` (`Set In` / `Set Out`)
- **Primary Route**: **Timeline Direct Manipulation**. Dragging the leading or trailing clip boundary handle (`Edge::Left` / `Edge::Right`) directly commits `SemanticEdit::Trim`.
- **Secondary Route**: **Shortcut with Active Source Locus**. When a `Source` clip is pointed, `[` and `]` (or `I` / `O`) sets in/out to the current playhead time.
- **Fail-Closed Behavior**: If no `Source` is pointed, `[` / `]` does not perform fallback; it speaks `"trim requires an active source binding"`.

### 2. `set-gain`
- **Primary Route**: **Audio / Source Property Inspector**. When a `Source` or `Audio` locus is committed, the Inspector exposes the `Gain (dB)` slider / scrub field.
- **Direct Route**: Inline gain handle overlay on the Timeline audio track clip.
- **Fail-Closed Behavior**: Disabled and absent when here is a `Title`, `Callout`, or unselected locus.

### 3. `set-fade`
- **Primary Route**: **Timeline Clip Edge Handles & Inspector**. Hovering the top corners of a `Source` clip reveals fade curve handles; editing commits `SemanticEdit::SetFade`.
- **Inspector Route**: `Fade In` / `Fade Out` duration fields in the Source Inspector when `LocusKind::Source` is active.

### 4. `split`
- **Primary Route**: **Timeline Playhead Knife Gesture / Shortcut (`Cmd+K` / `Ctrl+K` / `S`)**. When a `Scene` is active (or a `Source` clip's parent scene is addressed via Navigate), `Split` cuts the scene at the playhead position.
- **Secondary Route**: Right-click context menu on the Scene header or clip in Timeline: `Split Scene at Playhead`.
- **Fail-Closed Behavior**: If the playhead is outside the target scene's temporal span, `split` is rejected with Engine explanation `"playhead outside scene span"`.

### 5. `delete`
- **Primary Route**: **Keyboard Delete (`Backspace` / `Del`)**. Deletes the currently committed locus (`Scene`, `Title`, `Callout`).
- **Secondary Route**: Context menu on Timeline item or Sequence Tree node.
- **Fail-Closed Behavior**: If here is `None` or `UnresolvedPointing`, delete is a no-op.

---

## 4. Utterance: Disclosure-Only vs. Commit Surface

### Recommendation: **Disclosure-Only**
Under the Flash Chrome model, the **Utterance** is strictly **Disclosure-Only** (Read-Only Semantic Bar / Status Witness).

```text
+----------------------------------------------------------------------------------------------------+
| Utterance Bar [Disclosure-Only]:                                                                   |
| HERE: title Hello (main.vel:16)  |  LEGAL: title, set-position, resize-overlay  |  ROUTED: Canvas  |
| SPOKEN: "Drag canvas overlay to move. Drag corners to resize. Edit text in Inspector."             |
+----------------------------------------------------------------------------------------------------+
```

### Why Disclosure-Only?
1. **Separation of Concerns**: The Utterance acts as the voice of `Engine::legal_edits_for(locus)`. It tells the user and the external agent what is semantically true and where actions commit.
2. **Prevents UI Duplication**: Making the Utterance a click-to-commit surface would re-introduce the very "verb bank" problem we deleted, turning the utterance into a floating button strip that competes with direct manipulation instruments.
3. **Accessibility for External Agents**: The utterance is mirrored 1:1 in CLI `--json` and Studio `semantic_state.json`, ensuring parity between human and agent workflows without requiring GUI click orchestration.

---

## 5. Summary Matrix of Affordance Routing

| Verb | Engine Target | Committed On (Projection) | Gesture / Action | Utterance Spoken Clause |
|---|---|---|---|---|
| `title` | `LocusKind::Title` | **Inspector / Source** | Type text in Inspector / edit `.vel` | `title definition present in Inspector` |
| `set-position` | `LocusKind::Title`, `Callout` | **Canvas** | Click + Drag overlay body | `move in normalized Canvas space` |
| `resize-overlay` | `LocusKind::Title`, `Callout` | **Canvas** | Four-corner handle drag | `resize overlay preserving opposite corner` |
| `trim` | `LocusKind::Source` | **Timeline** | Drag clip edge handle (`Edge::Left/Right`) | `trim source range [in..out]` |
| `set-gain` | `LocusKind::Source` | **Inspector / Audio Track** | Gain slider / inline track handle | `set gain (dB) on source binding` |
| `set-fade` | `LocusKind::Source` | **Timeline / Inspector** | Top-corner fade handle drag | `set fade duration on source binding` |
| `split` | `LocusKind::Scene` | **Timeline / Keybinding** | Press `S` or `Cmd+K` at playhead | `split scene at source time t` |
| `delete` | `LocusKind::Scene`, `Title` | **Timeline / Tree / Key** | Press `Backspace` / `Delete` | `delete target locus from project` |
| `reorder-scene` | `LocusKind::Scene` | **Timeline** | Drag scene body past neighbor | `reorder scene index in sequence` |

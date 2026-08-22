(function () {
  "use strict";

  var LOCI = {
    title: {
      id: "demo:title:checkpoint",
      kind: "title",
      label: "Checkpoint",
      range: [1, 4]
    },
    scene: {
      id: "scene:demo",
      kind: "scene",
      label: "demo",
      range: [0, 10]
    },
    source: {
      id: "source:fight",
      kind: "source",
      label: "fight",
      range: [0, 10]
    }
  };

  var VEL = {
    title:
      'title "Checkpoint" {\n  at 1s for 3s\n  opacity 90\n}',
    scene:
      'scene demo {\n  game[10s..20s] as fight\n  freeze fight at 5.2s for 1.5s\n  title "Checkpoint" { at 1s for 3s }\n}',
    source:
      "media game \"capture.mp4\"\n\nscene demo {\n  game[10s..20s] as fight\n  freeze fight at 5.2s for 1.5s\n}"
  };

  var CAPTIONS = {
    a: "Open. Here is the title. The clock is at 0s. The title is active 1s–4s. That is temporal absence, not missing identity. Seek is explicit and keeps here.",
    b: "Point the identified video clip. Identity-bearing pointing commits that locus as here. The clock does not move.",
    c: "Scrub the audio rail across the title span. Scrub moves only the clock. It does not re-point here.",
    d: "Overlap. Title, scene, and source sit under the clock. An identified projection commits immediately. A bare coordinate opens a probe, then one shared locus.",
    e: "Legal verbs are Engine-named transitions from here. Each discloses verb, target, scope, and effect. If the Engine cannot name them, the verb is absent. No fallback off-here.",
    f: "Freeze is a rate-zero TimeMap hold with provenance. It is an explained temporal reading, not a selectable fake row.",
    g: "Review is an EditProposal bound to locus and revision. It can explain Title text and SetPosition. It is not a gate: direct commit remains legal.",
    h: "At about 800px the model does not change. Surfaces that cannot be drawn are named layout-unavailable. Here, clock, and verbs remain."
  };

  var state = {
    scene: "a",
    here: "title",
    playhead: 0,
    draft: null,
    probe: null,
    freezeOpen: false,
    reviewKind: "title",
    applied: null
  };

  function $(id) {
    return document.getElementById(id);
  }

  function clampTime(t) {
    return Math.max(0, Math.min(10, t));
  }

  function fmt(t) {
    return t.toFixed(1) + "s";
  }

  function active(locus, t) {
    return t >= locus.range[0] && t < locus.range[1];
  }

  function ledgerFor(hereKey, t) {
    var here = LOCI[hereKey];
    var rows = [];
    rows.push({
      status: "present",
      title: "Identity",
      body: here.id + " · " + here.kind + " · " + here.label
    });
    if (hereKey === "title") {
      rows.push({
        status: "present",
        title: "Source",
        body: "main.vel · title invocation · Navigate optional"
      });
      rows.push({
        status: active(here, t) ? "present" : "absent",
        title: "Timeline",
        body: active(here, t)
          ? "span 1.0s–4.0s · visible at clock"
          : "span 1.0s–4.0s · outside-playhead"
      });
      rows.push({
        status: active(here, t) ? "present" : "absent",
        title: "Visual",
        body: active(here, t)
          ? "placement present · text Checkpoint"
          : "geometry known · pixels not at " + fmt(t)
      });
      rows.push({
        status: "present",
        title: "Relations",
        body: "placement of scene demo · definition with one placement"
      });
    } else if (hereKey === "scene") {
      rows.push({
        status: "present",
        title: "Source",
        body: "main.vel · scene demo block"
      });
      rows.push({
        status: "present",
        title: "Timeline",
        body: "span 0s–10s · contains clock " + fmt(t)
      });
      rows.push({
        status: "struct",
        title: "Visual",
        body: "scene has no single overlay · observation is evaluate_at"
      });
      rows.push({
        status: "present",
        title: "Relations",
        body: "source fight · title Checkpoint · TimeMap on fight"
      });
    } else {
      rows.push({
        status: "present",
        title: "Source",
        body: "main.vel · game[10s..20s] as fight"
      });
      rows.push({
        status: "present",
        title: "Timeline",
        body: "span 0s–10s"
      });
      rows.push({
        status: "struct",
        title: "Visual",
        body: "not-applicable · media locus has no overlay of its own"
      });
      rows.push({
        status: "present",
        title: "Relations",
        body: "used by scene demo · TimeMap includes rate-zero hold"
      });
    }
    return rows;
  }

  function verbsFor(hereKey) {
    if (hereKey === "title") {
      return [
        {
          legal: true,
          verb: "Title",
          target: "demo:title:checkpoint",
          scope: "definition",
          effect: "rewrite the title invocation text or parameters"
        },
        {
          legal: true,
          verb: "SetPosition",
          target: "placement of Checkpoint",
          scope: "one placement",
          effect: "move this overlay in normalized canvas space"
        },
        {
          legal: true,
          verb: "ResizeOverlay",
          target: "placement of Checkpoint",
          scope: "one placement",
          effect: "scale this overlay, opposite corner fixed"
        },
        {
          legal: false,
          verb: "SetGain",
          target: "unnamed",
          scope: "none",
          effect: "Engine cannot name an audio target from this title. Verb absent. No fallback to fight."
        },
        {
          legal: false,
          verb: "ReorderScene",
          target: "unnamed",
          scope: "none",
          effect: "Engine cannot name a scene-order target from this title."
        }
      ];
    }
    if (hereKey === "scene") {
      return [
        {
          legal: true,
          verb: "ReorderScene",
          target: "scene:demo",
          scope: "scene order",
          effect: "move this scene inside sequence main"
        },
        {
          legal: false,
          verb: "Title",
          target: "unnamed",
          scope: "none",
          effect: "No title-shaped form. Scene is not a title definition."
        },
        {
          legal: false,
          verb: "SetPosition",
          target: "unnamed",
          scope: "none",
          effect: "Engine cannot name one placement from the scene locus."
        },
        {
          legal: false,
          verb: "Trim",
          target: "unnamed",
          scope: "none",
          effect: "Needs a stable clip identity. Scene here does not borrow fight's clip."
        },
        {
          legal: false,
          verb: "SetGain",
          target: "unnamed",
          scope: "none",
          effect: "Engine cannot name the exact SetGain target from scene demo. No hidden retarget to fight."
        }
      ];
    }
    return [
      {
        legal: true,
        verb: "SetGain",
        target: "source:fight",
        scope: "this source",
        effect: "set gain on the exact named source"
      },
      {
        legal: false,
        verb: "Title",
        target: "unnamed",
        scope: "none",
        effect: "Source is not a title definition."
      },
      {
        legal: false,
        verb: "ReorderScene",
        target: "unnamed",
        scope: "none",
        effect: "Source is not a scene in sequence order."
      }
    ];
  }

  function temporalFor(hereKey, t, freezeOpen) {
    var lines = [];
    lines.push("clock " + fmt(t));
    if (hereKey === "title") {
      if (t < 1) {
        lines.push("item local not yet in span 1.0s–4.0s");
      } else if (t >= 4) {
        lines.push("item local after span 1.0s–4.0s");
      } else {
        lines.push("item local " + (t - 1).toFixed(1) + "s of title span 1.0s–4.0s");
      }
      lines.push(active(LOCI.title, t) ? "content: title text is observable" : "content: outside-playhead");
    } else if (hereKey === "scene") {
      lines.push("item local " + fmt(t) + " of scene demo 0s–10s");
    } else {
      lines.push("item local " + fmt(t) + " of source fight");
    }
    if (t >= 5.2 && t < 6.7) {
      lines.push("TimeMap fight: timeline " + fmt(t) + " → item local " + fmt(t) + " → content 15.2s (held for 1.5s)");
      lines.push("rate 0 · provenance: freeze fight at 5.2s for 1.5s");
      if (freezeOpen) {
        lines.push("explained hold — not a LocusKind, not a tree identity");
      }
    } else if (hereKey === "source" || hereKey === "scene") {
      var content = 10 + t;
      lines.push("TimeMap fight: timeline " + fmt(t) + " → item local " + fmt(t) + " → content " + content.toFixed(1) + "s · rate 1");
    }
    return lines.join("\n");
  }

  function observation(hereKey, t) {
    var titleOn = active(LOCI.title, t);
    var held = t >= 5.2 && t < 6.7;
    var reason = null;
    if (hereKey === "title" && !titleOn) {
      reason = {
        code: "outside-playhead",
        text: "Here is title Checkpoint. Observed at " + fmt(t) + ". Visible range 1.0s–4.0s. Result: temporally absent.",
        seek: 1
      };
    }
    return { titleOn: titleOn, held: held, reason: reason, live: t < 10 };
  }

  function proposals() {
    return {
      title: {
        locus_id: "demo:title:checkpoint",
        base_revision: "a1f0c3e29b771004",
        edit: 'SemanticEdit::Title { text: "Hold the line" }',
        scope: "definition",
        effect: "rewrite the title invocation text",
        observation: "current picture still shows Checkpoint until Apply",
        diff:
          '- title "Checkpoint" {\n+ title "Hold the line" {\n    at 1s for 3s\n    opacity 90\n  }'
      },
      position: {
        locus_id: "demo:title:checkpoint",
        base_revision: "a1f0c3e29b771004",
        edit: "SemanticEdit::SetPosition { position: (0.62, 0.18) }",
        scope: "one placement",
        effect: "move this overlay in normalized canvas space",
        observation: "current picture still at the committed placement until Apply",
        diff:
          '  title "Checkpoint" {\n    at 1s for 3s\n+   position 0.62 0.18\n    opacity 90\n  }'
      }
    };
  }

  function renderLedger(t) {
    var ul = $("ledger-list");
    ul.innerHTML = "";
    ledgerFor(state.here, t).forEach(function (row) {
      var li = document.createElement("li");
      var tag = document.createElement("span");
      tag.className = "tag " + row.status;
      tag.textContent = row.status === "absent" ? "unavailable" : row.status === "struct" ? "not-applicable" : "present";
      li.appendChild(tag);
      li.appendChild(document.createTextNode(row.title));
      var meta = document.createElement("span");
      meta.className = "meta";
      meta.textContent = row.body;
      li.appendChild(meta);
      ul.appendChild(li);
    });
  }

  function verbItem(verb) {
    var li = document.createElement("li");
    li.className = verb.legal ? "legal" : "blocked";
    var tag = document.createElement("span");
    tag.className = "tag " + (verb.legal ? "verb" : "blocked");
    tag.textContent = verb.legal ? "legal" : "absent";
    li.appendChild(tag);
    li.appendChild(document.createTextNode(verb.verb));
    var meta = document.createElement("span");
    meta.className = "meta";
    meta.textContent =
      "target " + verb.target + " · scope " + verb.scope + " · " + verb.effect;
    li.appendChild(meta);
    return li;
  }

  function renderVerbList(ul, key) {
    ul.innerHTML = "";
    verbsFor(key).forEach(function (verb) {
      ul.appendChild(verbItem(verb));
    });
  }

  function renderVerbs() {
    var ul = $("verb-list");
    var compare = $("verb-compare");
    var split = state.scene === "e";
    compare.hidden = !split;
    ul.hidden = split;
    if (split) {
      compare.innerHTML =
        "<section><h3>From title Checkpoint</h3><ul id=\"verbs-title\"></ul></section>" +
        "<section><h3>From scene demo</h3><ul id=\"verbs-scene\"></ul></section>";
      renderVerbList($("verbs-title"), "title");
      renderVerbList($("verbs-scene"), "scene");
      return;
    }
    renderVerbList(ul, state.here);
  }

  function renderObservation(t) {
    var obs = observation(state.here, t);
    var footage = $("footage");
    var title = $("overlay-title");
    var hold = $("overlay-hold");
    var absence = $("observe-absence");
    footage.className =
      "footage" +
      (obs.held ? " is-held" : obs.live ? " is-live" : "") +
      (state.here === "scene" ? " is-here" : "");
    title.hidden = !obs.titleOn || !!obs.reason;
    title.classList.toggle("is-here", state.here === "title" && obs.titleOn);
    hold.hidden = !obs.held;
    $("clip-plate").hidden = !!obs.reason;
    if (obs.reason) {
      absence.hidden = false;
      absence.innerHTML =
        '<p class="reason">' +
        obs.reason.code +
        "</p><p>" +
        obs.reason.text +
        '</p><button type="button" data-action="seek">Seek to ' +
        fmt(obs.reason.seek) +
        "</button>";
    } else {
      absence.hidden = true;
      absence.innerHTML = "";
    }
  }

  function proposalCard(p, heading) {
    return (
      "<article><h3>" +
      heading +
      "</h3><p><code>locus_id</code> " +
      p.locus_id +
      "</p><p><code>base_revision</code> " +
      p.base_revision +
      "</p><p>" +
      p.edit +
      "</p><p>scope " +
      p.scope +
      " · " +
      p.effect +
      "</p><p>" +
      p.observation +
      "</p><pre>" +
      p.diff +
      "</pre></article>"
    );
  }

  function renderReview() {
    var card = $("review-card");
    var show = state.scene === "g";
    card.hidden = !show;
    if (!show) {
      if (state.scene !== "g") state.draft = null;
      return;
    }
    var kind = state.reviewKind;
    $("review-body").setAttribute("data-cols", kind === "both" ? "3" : "2");
    document.querySelectorAll(".review-switch button").forEach(function (btn) {
      var act = btn.getAttribute("data-action");
      btn.classList.toggle(
        "is-on",
        (kind === "both" && act === "review-both") ||
          (kind === "title" && act === "review-title") ||
          (kind === "position" && act === "review-position") ||
          (kind === "direct" && act === "review-direct")
      );
    });
    var body = $("review-body");
    var all = proposals();
    if (kind === "direct") {
      body.innerHTML =
        "<article><h3>Direct</h3><p>Same verb <code>Title</code>, same target <code>demo:title:checkpoint</code>, same definition scope.</p><p>Committed immediately through the Engine. One rewrite, one compile, one Undo. Review was not opened.</p><p class=\"meta\">Source now holds Hold the line. No proposal remains.</p></article>" +
        "<article><h3>Policy</h3><p>Proposal versus direct is a workflow choice, not an edit-kind gate. Title text and SetPosition may take either path.</p></article>";
      state.draft = null;
      state.applied = "direct-title";
      return;
    }
    if (kind === "both") {
      state.draft = all.title;
      body.innerHTML =
        proposalCard(all.title, "After Title text") +
        proposalCard(all.position, "After SetPosition") +
        (state.applied === "direct-title"
          ? '<article><h3>Apply</h3><p class="meta">stale-proposal · base_revision no longer matches current source. Studio does not retarget.</p></article>'
          : '<article><h3>Apply is not a gate</h3><p>Either proposal keeps current source until Apply. Direct Title text remains legal.</p><div class="review-actions"><button type="button" class="apply" data-action="apply-proposal">Apply Title text</button><button type="button" class="reject" data-action="reject-proposal">Reject</button></div></article>');
      return;
    }
    var p = all[kind];
    state.draft = p;
    var stale = state.applied === "direct-title";
    body.innerHTML =
      proposalCard(p, "Proposal") +
      "<article><h3>Current observation</h3><p>" +
      p.observation +
      "</p>" +
      (stale
        ? '<p class="meta">stale-proposal · base_revision no longer matches current source. Studio does not retarget.</p>'
        : '<div class="review-actions"><button type="button" class="apply" data-action="apply-proposal">Apply</button><button type="button" class="reject" data-action="reject-proposal">Reject</button></div>') +
      "</article>";
  }

  function renderProbe() {
    var sheet = $("probe-sheet");
    var open = !!state.probe;
    sheet.hidden = !open;
    $("probe-card").classList.toggle("is-on", open);
    $("probe-readout").textContent = open ? state.probe.length + " candidates" : "none";
    if (!open) {
      $("probe-list").innerHTML = "";
      return;
    }
    var ul = $("probe-list");
    ul.innerHTML = "";
    state.probe.forEach(function (key) {
      var locus = LOCI[key];
      var li = document.createElement("li");
      li.setAttribute("data-action", "commit-" + key);
      li.innerHTML =
        "<strong>" +
        locus.kind +
        " · " +
        locus.label +
        "</strong><span class=\"meta\"><code>" +
        locus.id +
        "</code> · span " +
        locus.range[0] +
        "s–" +
        locus.range[1] +
        "s · choose to commit the shared locus</span>";
      ul.appendChild(li);
    });
  }

  function renderNarrow() {
    var card = $("narrow-card");
    var narrow = state.scene === "h";
    card.hidden = !narrow;
    document.body.setAttribute("data-narrow", narrow ? "true" : "false");
    if (!narrow) return;
    $("narrow-list").innerHTML =
      "<li><span class=\"tag present\">stays</span>Committed LocusId, playhead, legal verbs, typed absences</li>" +
      "<li><span class=\"tag absent\">goes</span>Observation picture, full source listing, hit-contract chips</li>" +
      "<li><span class=\"tag struct\">named</span><code>layout-unavailable</code> · observation surface cannot be drawn at this width</li>" +
      "<li><span class=\"tag struct\">named</span><code>layout-unavailable</code> · source reading collapsed; provenance remains on the ledger</li>" +
      "<li><span class=\"tag present\">unchanged</span>Here is still " +
      LOCI[state.here].id +
      ". Width does not mint a second selection.</li>";
  }

  function render() {
    var here = LOCI[state.here];
    var t = state.playhead;
    document.body.setAttribute("data-scene", state.scene);
    document.body.setAttribute("data-here", state.here);
    document.body.setAttribute("data-playhead", String(t));
    $("here-id").textContent = here.id;
    $("here-name").textContent = here.label;
    $("here-kind").textContent = here.kind;
    $("scene-caption").textContent = CAPTIONS[state.scene];
    $("playhead-readout").textContent = fmt(t);
    $("playhead-line").style.left = "calc(64px + (100% - 64px) * " + t / 10 + ")";
    $("clip-title").classList.toggle("is-here", state.here === "title");
    $("clip-scene").classList.toggle("is-here", state.here === "scene");
    $("temporal-readout").textContent = temporalFor(state.here, t, state.freezeOpen);
    $("source-readout").textContent = VEL[state.here] || VEL.scene;
    $("freeze-mark").classList.toggle("is-open", state.freezeOpen);
    document.querySelectorAll(".walk a").forEach(function (a) {
      a.classList.toggle("is-current", a.getAttribute("data-go") === state.scene);
    });
    renderLedger(t);
    renderVerbs();
    renderObservation(t);
    renderReview();
    renderProbe();
    renderNarrow();
    $("draft-card").classList.toggle("is-on", !!state.draft);
    $("draft-readout").textContent = state.draft
      ? state.reviewKind === "position"
        ? "SetPosition"
        : "Title text"
      : "none";
  }

  function enterScene(scene) {
    state.scene = scene;
    state.probe = null;
    state.freezeOpen = false;
    state.applied = null;
    state.draft = null;
    state.reviewKind = "title";
    if (scene === "a") {
      state.here = "title";
      state.playhead = 0;
    } else if (scene === "b") {
      state.here = "scene";
      state.playhead = 0;
    } else if (scene === "c") {
      state.here = "scene";
      state.playhead = 2.4;
    } else if (scene === "d") {
      state.here = "scene";
      state.playhead = 2.4;
      state.probe = ["title", "scene", "source"];
    } else if (scene === "e") {
      state.here = "title";
      state.playhead = 2.4;
    } else if (scene === "f") {
      state.here = "source";
      state.playhead = 5.2;
      state.freezeOpen = true;
    } else if (scene === "g") {
      state.here = "title";
      state.playhead = 2.4;
      state.reviewKind = "both";
    } else if (scene === "h") {
      state.here = "title";
      state.playhead = 0;
    }
    if (location.hash !== "#" + scene) {
      history.replaceState(null, "", "#" + scene);
    }
    render();
  }

  function commitHere(key) {
    state.here = key;
    state.probe = null;
    render();
  }

  function seek(t) {
    state.playhead = clampTime(t);
    render();
  }

  function openProbe() {
    state.probe = ["title", "scene", "source"];
    render();
  }

  function actionFrom(el) {
    while (el && el !== document.body) {
      if (el.getAttribute && el.getAttribute("data-action")) {
        return el.getAttribute("data-action");
      }
      el = el.parentElement;
    }
    return null;
  }

  function onClick(event) {
    var action = actionFrom(event.target);
    if (!action) return;
    if (action === "seek") {
      seek(1);
      return;
    }
    if (action === "point-title") {
      commitHere("title");
      return;
    }
    if (action === "point-scene") {
      commitHere("scene");
      return;
    }
    if (action === "point-source") {
      commitHere("source");
      return;
    }
    if (action === "scrub-audio") {
      var rail = $("audio-rail");
      var rect = rail.getBoundingClientRect();
      var x = event.clientX - rect.left;
      seek((x / rect.width) * 10);
      return;
    }
    if (action === "probe-coord") {
      openProbe();
      return;
    }
    if (action === "cancel-probe") {
      state.probe = null;
      render();
      return;
    }
    if (action === "commit-title") {
      commitHere("title");
      return;
    }
    if (action === "commit-scene") {
      commitHere("scene");
      return;
    }
    if (action === "commit-source") {
      commitHere("source");
      return;
    }
    if (action === "explain-freeze") {
      state.freezeOpen = !state.freezeOpen;
      render();
      return;
    }
    if (action === "review-both") {
      state.reviewKind = "both";
      render();
      return;
    }
    if (action === "review-title") {
      state.reviewKind = "title";
      render();
      return;
    }
    if (action === "review-position") {
      state.reviewKind = "position";
      render();
      return;
    }
    if (action === "review-direct") {
      state.reviewKind = "direct";
      render();
      return;
    }
    if (action === "apply-proposal") {
      state.applied = state.reviewKind;
      state.draft = null;
      $("review-body").innerHTML =
        "<article><h3>Applied</h3><p>Apply succeeded because <code>locus_id</code> still names the target and <code>base_revision</code> matches current source.</p></article>";
      $("draft-readout").textContent = "none";
      $("draft-card").classList.remove("is-on");
      return;
    }
    if (action === "reject-proposal") {
      state.draft = null;
      state.reviewKind = "title";
      render();
      return;
    }
    if (action.indexOf("go") === 0) return;
  }

  document.querySelectorAll("[data-go]").forEach(function (a) {
    a.addEventListener("click", function (event) {
      event.preventDefault();
      enterScene(a.getAttribute("data-go"));
    });
  });

  document.body.addEventListener("click", onClick);

  window.addEventListener("hashchange", function () {
    var scene = (location.hash || "#a").slice(1);
    if (CAPTIONS[scene] && scene !== state.scene) enterScene(scene);
  });

  var initial = (location.hash || "#a").slice(1);
  enterScene(CAPTIONS[initial] ? initial : "a");
})();

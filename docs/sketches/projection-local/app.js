(function () {
  "use strict";

  var captions = {
    a: {
      letter: "A",
      title: "Open · title here, playhead 0s",
      body: "title Hello is the verb subject. The playhead is 0.00s. The title is active 1s–4s, so Canvas has no picture. That projection names the cause and offers a local, temporary seek. Pointing did not move time. Emptiness did not move the locus."
    },
    b: {
      letter: "B",
      title: "Video clip click · scene here",
      body: "The Video scene body was the touched projection. Here is scene demo. Timeline offers ReorderScene and scene-scoped Delete. It does not offer SetPosition, and it does not borrow a title form. Source shows quiet identity only."
    },
    c: {
      letter: "C",
      title: "Scrub Audio rail across the title span",
      body: "The Audio rail background is scrub only. Playhead is now 2.40s, inside the title range, so Canvas has a picture. Here is still title Hello — scrub did not re-point. There is no time-scoped audio SemanticEdit; this rail does not invent one."
    },
    d: {
      letter: "D",
      title: "Overlap · failed point on Timeline",
      body: "At 2.40s the playhead sits in title Hello, scene demo, and source fight. Pointing that pixel failed to name a unique subject. Candidates appear on this Timeline with reason and scope. A choice commits one LocusId everywhere."
    },
    e: {
      letter: "E",
      title: "License = touched projection ∩ Engine-legal",
      body: "The same product, two instruments. Timeline ∩ scene licenses ReorderScene and refuses SetPosition. Canvas ∩ title placement licenses SetPosition / ResizeOverlay and refuses ReorderScene. Definition text stays on the source span."
    },
    f: {
      letter: "F",
      title: "Freeze is not a selectable row",
      body: "Here is source fight. Canvas shows the held frame at 5.20s because evaluate_at sees TimeMap rate 0. The rate-0 segment is an explanation, not a locus. No synthetic freeze identity, no verb surface, no Core Freeze."
    },
    g: {
      letter: "G",
      title: "Review after text and after SetPosition",
      body: "Review is a projection of an EditProposal. Left: definition-scope Title { text } from the source span. Right: this-placement SetPosition from the Canvas that licenses it. Apply / Reject only. Review does not begin a new gesture and is not a title-only text pane."
    },
    h: {
      letter: "H",
      title: "Narrow ~800px",
      body: "Canvas, Timeline, session facts, transport, and the local seek stay. Source / VEL leaves the frame and is named absent, with Navigate still offered. There is no Inspector to collapse. Width is a visibility constraint, not a verb license."
    }
  };

  var scenes = {
    a: { locus: "title", playhead: 0, touched: "Canvas", touchNote: "no picture · local seek only", whenNote: "outside title 1s–4s" },
    b: { locus: "scene", playhead: 0, touched: "Timeline", touchNote: "scene body · point + reorder", whenNote: "evaluate_at still 0s" },
    c: { locus: "title", playhead: 2.4, touched: "Audio rail", touchNote: "scrub only · not a subject", whenNote: "inside title 1s–4s" },
    d: { locus: "overlap", playhead: 2.4, touched: "Timeline", touchNote: "failed point · candidate list", whenNote: "title + scene + source" },
    e: { locus: "both", playhead: 2.4, touched: "Timeline and Canvas", touchNote: "each licenses its own verbs", whenNote: "picture present for title" },
    f: { locus: "source", playhead: 5.2, touched: "Source · TimeMap", touchNote: "explain rate 0 · no verbs", whenNote: "held frame at 5.20s" },
    g: { locus: "title", playhead: 2.4, touched: "Review", touchNote: "Apply / Reject of that proposal", whenNote: "evidence, not a gate" },
    h: { locus: "title", playhead: 0, touched: "Canvas", touchNote: "local seek still here", whenNote: "outside title 1s–4s" }
  };

  var loci = {
    title: { label: "title Hello", id: "loc_title_hello" },
    scene: { label: "scene demo", id: "loc_scene_demo" },
    source: { label: "source fight", id: "loc_source_fight" },
    overlap: { label: "unresolved · several loci", id: "— pending choice —" },
    both: { label: "scene demo  ·  title Hello", id: "loc_scene_demo  ·  loc_title_hello" }
  };

  var state = { scene: "hub", playhead: 0, chosen: "" };

  function $(sel, root) {
    return (root || document).querySelector(sel);
  }

  function $all(sel, root) {
    return Array.prototype.slice.call((root || document).querySelectorAll(sel));
  }

  function setText(sel, text) {
    $all(sel).forEach(function (el) { el.textContent = text; });
  }

  function formatTime(t) {
    return t.toFixed(2) + "s";
  }

  function placePlayheads(t) {
    var pct = Math.max(0, Math.min(100, (t / 10) * 100));
    $all("[data-playhead]").forEach(function (el) {
      el.style.left = pct + "%";
    });
  }

  function applyLocus(key) {
    var loc = loci[key] || loci.title;
    setText("[data-bind='here-label']", loc.label);
    setText("[data-bind='here-id']", loc.id);
    setText("[data-bind='choice-id']", key === "overlap" ? "one LocusId" : loc.id);
  }

  function applyScene(id) {
    if (id === "hub" || !scenes[id]) {
      document.body.dataset.scene = "hub";
      document.body.removeAttribute("data-chosen");
      state.scene = "hub";
      if (location.hash && location.hash !== "#hub") {
        history.replaceState(null, "", "#hub");
      }
      return;
    }

    var cfg = scenes[id];
    state.scene = id;
    state.playhead = cfg.playhead;
    state.chosen = "";
    document.body.dataset.scene = id;
    document.body.removeAttribute("data-chosen");

    applyLocus(cfg.locus);
    setText("[data-bind='when-label']", formatTime(cfg.playhead));
    setText("[data-bind='when-note']", cfg.whenNote);
    setText("[data-bind='touch-label']", cfg.touched);
    setText("[data-bind='touch-note']", cfg.touchNote);
    setText("[data-bind='clock']", formatTime(cfg.playhead));
    setText("[data-bind='choice-note']", "The failed point stays visible until that choice.");

    var cap = captions[id];
    setText("[data-bind='cap-letter']", cap.letter);
    setText("[data-bind='cap-title']", cap.title);
    setText("[data-bind='cap-body']", cap.body);

    $all(".proj, .timeline").forEach(function (el) {
      el.classList.remove("touched");
    });
    if (id === "a" || id === "h") $("#proj-canvas").classList.add("touched");
    if (id === "b" || id === "d") $("#timeline").classList.add("touched");
    if (id === "c") $("#timeline").classList.add("touched");
    if (id === "f") $("#proj-source").classList.add("touched");

    placePlayheads(cfg.playhead);

    if (location.hash !== "#" + id) {
      history.replaceState(null, "", "#" + id);
    }
  }

  function timeFromEvent(event, el) {
    var rect = el.getBoundingClientRect();
    var x = event.clientX - rect.left;
    return Math.max(0, Math.min(10, (x / rect.width) * 10));
  }

  function onScrub(event) {
    if (state.scene !== "c" && state.scene !== "a" && state.scene !== "h") return;
    if (state.scene !== "c" && !event.currentTarget.hasAttribute("data-scrub")) return;
    var t = timeFromEvent(event, event.currentTarget);
    if (state.scene === "c" || event.currentTarget.getAttribute("data-scrub") === "audio") {
      state.playhead = t;
      setText("[data-bind='when-label']", formatTime(t));
      setText("[data-bind='clock']", formatTime(t));
      placePlayheads(t);
      var inside = t >= 1 && t <= 4;
      setText("[data-bind='when-note']", inside ? "inside title 1s–4s · here unchanged" : "outside title · here unchanged");
    }
  }

  function pickCandidate(kind) {
    if (state.scene !== "d") return;
    state.chosen = kind;
    document.body.dataset.chosen = kind;
    applyLocus(kind);
    setText("[data-bind='touch-note']", "choice committed · shared LocusId");
    setText("[data-bind='choice-note']", "Every projection now holds " + loci[kind].id + ".");
    setText("[data-bind='here-label']", loci[kind].label);
  }

  document.addEventListener("click", function (event) {
    var go = event.target.closest("[data-go], a[href^='#']");
    if (go) {
      var id = go.getAttribute("data-go") || (go.getAttribute("href") || "").replace("#", "");
      if (id) {
        event.preventDefault();
        applyScene(id);
      }
      return;
    }
    var seek = event.target.closest("[data-seek]");
    if (seek && (state.scene === "a" || state.scene === "h")) {
      state.playhead = 1;
      setText("[data-bind='when-label']", "1.00s");
      setText("[data-bind='clock']", "1.00s");
      setText("[data-bind='when-note']", "explicit seek · not persisted");
      placePlayheads(1);
      return;
    }
    var pick = event.target.closest("[data-pick]");
    if (pick) {
      pickCandidate(pick.getAttribute("data-pick"));
      return;
    }
    var point = event.target.closest("[data-point]");
    if (point && state.scene === "b") {
      applyLocus(point.getAttribute("data-point"));
    }
  });

  $all("[data-scrub]").forEach(function (el) {
    el.addEventListener("pointerdown", function (event) {
      if (state.scene !== "c") return;
      onScrub(event);
      function move(ev) { onScrub(ev); }
      function up() {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", up);
      }
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", up);
    });
  });

  window.addEventListener("hashchange", function () {
    applyScene((location.hash || "#hub").slice(1));
  });

  applyScene((location.hash || "#hub").slice(1) || "hub");
})();

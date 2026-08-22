(function () {
  function $(sel, root) {
    return (root || document).querySelector(sel);
  }
  function $all(sel, root) {
    return Array.prototype.slice.call((root || document).querySelectorAll(sel));
  }

  function setPlayhead(seconds, label) {
    $all("[data-playhead-label]").forEach(function (el) {
      el.textContent = label || seconds.toFixed(1) + "s";
    });
    $all("[data-playhead]").forEach(function (el) {
      var duration = parseFloat(el.getAttribute("data-duration") || "10");
      var pct = Math.max(0, Math.min(100, (seconds / duration) * 100));
      el.style.left = pct + "%";
    });
    $all("[data-when]").forEach(function (el) {
      var start = parseFloat(el.getAttribute("data-span-start") || "1");
      var end = parseFloat(el.getAttribute("data-span-end") || "4");
      var inside = seconds >= start && seconds < end;
      if (inside) {
        el.innerHTML =
          "active " +
          start +
          "s–" +
          end +
          "s; you are at <span class=\"mono\">" +
          seconds.toFixed(1) +
          "s</span> <em>(inside)</em>";
      } else if (seconds < start) {
        el.innerHTML =
          "active " +
          start +
          "s–" +
          end +
          "s; you are at <span class=\"mono\">" +
          seconds.toFixed(1) +
          "s</span> <em>(before it starts)</em>";
      } else {
        el.innerHTML =
          "active " +
          start +
          "s–" +
          end +
          "s; you are at <span class=\"mono\">" +
          seconds.toFixed(1) +
          "s</span> <em>(after it ends)</em>";
      }
    });
    var seek = $("[data-seek]");
    if (seek) {
      var start = parseFloat(seek.getAttribute("data-span-start") || "1");
      var end = parseFloat(seek.getAttribute("data-span-end") || "4");
      var inside = seconds >= start && seconds < end;
      seek.hidden = inside;
      $all("[data-temporal]").forEach(function (el) {
        el.hidden = inside;
      });
    }
    var stage = $("[data-stage]");
    if (stage) {
      var s0 = parseFloat(stage.getAttribute("data-span-start") || "1");
      var s1 = parseFloat(stage.getAttribute("data-span-end") || "4");
      var on = seconds >= s0 && seconds < s1;
      stage.classList.toggle("no-frame", !on && stage.hasAttribute("data-empty-before"));
      stage.classList.toggle("game", on || !stage.hasAttribute("data-empty-before"));
      var overlay = $(".overlay", stage);
      if (overlay && stage.hasAttribute("data-empty-before")) {
        overlay.classList.toggle("ghost", !on);
      }
    }
  }

  $all("[data-seek]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var t = parseFloat(btn.getAttribute("data-span-start") || "1");
      setPlayhead(t, t.toFixed(1) + "s");
    });
  });

  $all("[data-scrub]").forEach(function (track) {
    function timeAt(ev) {
      var rect = track.getBoundingClientRect();
      var x = (ev.clientX - rect.left) / rect.width;
      var duration = parseFloat(track.getAttribute("data-duration") || "10");
      return Math.max(0, Math.min(duration, x * duration));
    }
    function move(ev) {
      var t = timeAt(ev);
      setPlayhead(t, t.toFixed(1) + "s");
      var note = $("[data-scrub-note]");
      if (note) note.hidden = false;
    }
    track.addEventListener("pointerdown", function (ev) {
      track.classList.add("scrubbing");
      track.setPointerCapture(ev.pointerId);
      move(ev);
    });
    track.addEventListener("pointermove", function (ev) {
      if (!track.classList.contains("scrubbing")) return;
      move(ev);
    });
    track.addEventListener("pointerup", function () {
      track.classList.remove("scrubbing");
    });
  });

  $all("[data-jump]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var t = parseFloat(btn.getAttribute("data-jump"));
      setPlayhead(t, t.toFixed(1) + "s");
    });
  });

  var stepIndex = 0;
  var stepBtns = $all("[data-step-to]");
  if (stepBtns.length) {
    function showStep(i) {
      stepIndex = (i + stepBtns.length) % stepBtns.length;
      $all("[data-step-panel]").forEach(function (panel) {
        panel.hidden = panel.getAttribute("data-step-panel") !== String(stepIndex);
      });
      $all("[data-cand]").forEach(function (el) {
        el.classList.toggle("taken", el.getAttribute("data-cand") === String(stepIndex));
      });
      $all("[data-step-locus]").forEach(function (el) {
        el.classList.toggle("here", el.getAttribute("data-step-locus") === String(stepIndex));
      });
    }
    $all("[data-step]").forEach(function (btn) {
      btn.addEventListener("click", function () {
        showStep(stepIndex + 1);
      });
    });
    stepBtns.forEach(function (btn) {
      btn.addEventListener("click", function () {
        showStep(parseInt(btn.getAttribute("data-step-to"), 10));
      });
    });
  }

  $all("[data-open-verb]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var id = btn.getAttribute("data-open-verb");
      $all("[data-verb-field]").forEach(function (field) {
        field.hidden = field.getAttribute("data-verb-field") !== id;
      });
    });
  });
  $all("[data-dismiss-verb]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      $all("[data-verb-field]").forEach(function (field) {
        field.hidden = true;
      });
    });
  });
})();

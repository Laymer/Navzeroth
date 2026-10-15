/* Navzeroth site behaviour: feature tour, FAQ, Atlas layer demo, scroll reveal.
   No dependencies and no network requests; everything on the page is static. */
(function () {
  'use strict';

  var reduced = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var animated = !reduced;
  var $ = function (sel, root) { return (root || document).querySelector(sel); };
  var $$ = function (sel, root) { return Array.prototype.slice.call((root || document).querySelectorAll(sel)); };

  /* ---------- Feature tour ---------- */
  var tourViewport = $('#tour-viewport');
  var tourDoc = $('#tour-doc');
  var hudViewport = $('#hud-viewport');
  var hudPlate = hudViewport && hudViewport.querySelector('div[style*="width: 420px"]');
  var tourWindow = $('.tour-window');
  var tabs = $$('.tour-tab');
  var selected = 'atlas';
  var shown = 'atlas';
  var timer = 0;

  function measure() {
    if (tourViewport && tourViewport.clientWidth > 0) {
      var s = tourViewport.clientWidth / 820;
      tourViewport.style.height = Math.round(600 * s) + 'px';
      tourViewport.firstElementChild.style.transform = 'scale(' + s + ')';
    }
    if (hudViewport && hudViewport.clientWidth > 0 && hudPlate) {
      var h = hudViewport.clientWidth / 760;
      hudPlate.style.top = Math.round(120 * h) + 'px';
      hudPlate.style.transform = 'translateX(-50%) scale(' + h + ')';
    }
    if (tourDoc) {
      // Atlas scrolls so the map's bottom sits 54px above the window bottom;
      // Dungeons scrolls so its heading is 14px from the top; the rest start at the top.
      var screen = $('[data-screen]:not([hidden])', tourDoc);
      var map = screen && $('[data-anchor="map"]', screen);
      var top = screen && $('[data-anchor="top"]', screen);
      var off = 0;
      if (map) off = map.offsetTop + map.offsetHeight + 54 - 600;
      else if (top) off = top.offsetTop - 14;
      tourDoc.style.transform = 'translateY(' + -Math.max(0, Math.round(off)) + 'px)';
    }
  }

  function remeasure() {
    requestAnimationFrame(measure);
    setTimeout(measure, 60);
  }

  function swap(id) {
    shown = id;
    $$('[data-stage]').forEach(function (el) { el.hidden = el.getAttribute('data-stage') !== (id === 'hud' ? 'hud' : 'app'); });
    $$('[data-screen]').forEach(function (el) { el.hidden = el.getAttribute('data-screen') !== id; });
    $$('[data-caption]').forEach(function (el) { el.hidden = el.getAttribute('data-caption') !== id; });
    if (tourWindow) tourWindow.classList.remove('is-fading');
    measure();
    remeasure();
  }

  function pick(id) {
    if (id === shown && selected === id) return;
    selected = id;
    tabs.forEach(function (tab) { tab.setAttribute('aria-selected', String(tab.getAttribute('data-pick') === id)); });
    clearTimeout(timer);
    if (animated && tourWindow) {
      tourWindow.classList.add('is-fading');
      timer = setTimeout(function () { swap(id); }, 170);
    } else {
      swap(id);
    }
  }

  document.addEventListener('click', function (e) {
    var trigger = e.target.closest('[data-pick]');
    if (trigger) pick(trigger.getAttribute('data-pick'));
  });

  /* ---------- Atlas layer demo (Flight replaces Roads; neither shows the overview) ---------- */
  var kinds = { flight: 'flight reference', roads: 'roads traced', overview: 'overview' };
  function updateLayers() {
    var roads = $('[data-layer-toggle="roads"]');
    var flight = $('[data-layer-toggle="flight"]');
    if (!roads || !flight) return;
    var layer = flight.checked ? 'flight' : roads.checked ? 'roads' : 'overview';
    $$('[data-layer]').forEach(function (el) { el.hidden = el.getAttribute('data-layer') !== layer; });
    var name = $('[data-layer-name]');
    var kind = $('[data-layer-kind]');
    if (name) name.textContent = layer;
    if (kind) kind.textContent = kinds[layer];
  }
  $$('[data-layer-toggle]').forEach(function (box) { box.addEventListener('change', updateLayers); });

  /* ---------- FAQ: one toggle per item, several can be open ---------- */
  $$('[data-faq]').forEach(function (btn) {
    btn.addEventListener('click', function () {
      btn.setAttribute('aria-expanded', String(btn.getAttribute('aria-expanded') !== 'true'));
    });
  });

  /* ---------- Hero embers and backdrop scrim ---------- */
  var embers = $('#embers');
  if (embers && animated) {
    for (var i = 0; i < 16; i++) {
      var r = function (n) { var v = Math.sin(i * 12.9898 + n * 78.233) * 43758.5453; return v - Math.floor(v); };
      var size = 2 + Math.round(r(2) * 2);
      var dot = document.createElement('span');
      dot.style.cssText = 'position: absolute; left: ' + (46 + r(1) * 52) + '%; bottom: -12px; width: ' + size + 'px; height: ' + size +
        'px; border-radius: 50%; background: var(--color-accent-300); box-shadow: 0 0 ' + (size * 3) + 'px var(--color-accent); --dx: ' +
        Math.round((r(3) - 0.5) * 90) + 'px; opacity: 0; animation: nz-ember ' + (9 + r(4) * 8) + 's linear ' + (-r(5) * 14) + 's infinite;';
      embers.appendChild(dot);
    }
  }

  var scrim = $('#scrim');
  var scrollRaf = 0;
  function applyScrim() {
    if (!scrim) return;
    var vh = window.innerHeight || 800;
    var p = Math.min(1, Math.max(0, (window.scrollY || 0) / (vh * 0.7)));
    scrim.style.opacity = (p * 0.5).toFixed(3);
  }
  function onScroll() {
    if (scrollRaf) return;
    scrollRaf = requestAnimationFrame(function () { scrollRaf = 0; applyScrim(); });
  }
  window.addEventListener('scroll', onScroll, { passive: true });
  window.addEventListener('resize', onScroll);
  applyScrim();

  /* ---------- Scroll reveal: blocks that start below the fold fade up ---------- */
  if (animated && 'IntersectionObserver' in window) {
    var vh = window.innerHeight || 800;
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (!entry.isIntersecting) return;
        entry.target.style.opacity = '1';
        entry.target.style.transform = 'none';
        io.unobserve(entry.target);
      });
    }, { rootMargin: '0px 0px -6% 0px' });
    $$('[data-reveal]').forEach(function (el) {
      if (el.getBoundingClientRect().top < vh * 0.92) return;
      el.style.opacity = '0';
      el.style.transform = 'translateY(18px)';
      el.style.transition = 'opacity 700ms ease, transform 760ms cubic-bezier(.2,.7,.2,1)';
      io.observe(el);
    });
  }

  /* ---------- Sizing ---------- */
  if ('ResizeObserver' in window) {
    var ro = new ResizeObserver(measure);
    [tourViewport, hudViewport].forEach(function (el) { if (el) ro.observe(el); });
  }
  window.addEventListener('resize', measure);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(measure);
  measure();
})();

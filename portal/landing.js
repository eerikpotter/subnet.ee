const { t } = window.SubnetI18n;
/* The public page loads only lightweight navigation; account/Wasm lives in /tooruum/. */
const oldWorkspaceHashes = new Set(['#failid', '#rakendused']);
function followLegacyLink() {
  if (oldWorkspaceHashes.has(location.hash)) location.replace(window.SubnetI18n.href(`/tooruum/${location.search}${location.hash}`));
}
followLegacyLink();
window.addEventListener('hashchange', followLegacyLink);

const menuToggle = document.querySelector('.menu-toggle');
const navigation = document.getElementById('main-nav');
function closeMenu() {
  navigation.classList.remove('is-open');
  menuToggle.setAttribute('aria-expanded', 'false');
  menuToggle.setAttribute('aria-label', t('Ava menüü'));
}
menuToggle.addEventListener('click', () => {
  const open = menuToggle.getAttribute('aria-expanded') !== 'true';
  navigation.classList.toggle('is-open', open);
  menuToggle.setAttribute('aria-expanded', String(open));
  menuToggle.setAttribute('aria-label', open ? t('Sulge menüü') : t('Ava menüü'));
});
window.addEventListener('subnet:languagechange', () => {
  menuToggle.setAttribute('aria-label', t(menuToggle.getAttribute('aria-expanded') === 'true' ? 'Sulge menüü' : 'Ava menüü'));
});
navigation.addEventListener('click', (event) => { if (event.target.closest('a')) closeMenu(); });
document.addEventListener('click', (event) => { if (!event.target.closest('.site-header')) closeMenu(); });
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && menuToggle.getAttribute('aria-expanded') === 'true') {
    closeMenu();
    menuToggle.focus();
  }
});
window.matchMedia('(min-width: 861px)').addEventListener('change', closeMenu);

// Native scrolling keeps every app reachable with touch, a trackpad or the keyboard.
// Automatic motion gently reverses at either end; no duplicate links or jump cuts.
const toolsCarousel = document.querySelector('.tools-carousel');
const toolsTrack = document.getElementById('tools-track');
if (toolsCarousel && toolsTrack) {
  const previous = toolsCarousel.querySelector('[data-tools-previous]');
  const next = toolsCarousel.querySelector('[data-tools-next]');
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
  let hovered = false;
  let visible = false;
  let interacting = false;
  let resumeAfter = 0;
  let direction = 1;
  let position = toolsTrack.scrollLeft;
  let lastFrame = 0;
  let drag = null;
  let dragged = false;
  const end = () => Math.max(0, toolsTrack.scrollWidth - toolsTrack.clientWidth);
  const hold = () => { resumeAfter = performance.now() + 1200; };
  function updateControls() {
    previous.disabled = toolsTrack.scrollLeft <= 1;
    next.disabled = toolsTrack.scrollLeft >= end() - 1;
  }
  function move(amount) {
    hold();
    toolsTrack.scrollBy({ left: amount, behavior: reducedMotion.matches ? 'instant' : 'smooth' });
  }
  function step() {
    const card = toolsTrack.querySelector('.tool-card');
    return card ? card.getBoundingClientRect().width + parseFloat(getComputedStyle(toolsTrack).columnGap) : 200;
  }
  previous.addEventListener('click', () => move(-step()));
  next.addEventListener('click', () => move(step()));
  toolsCarousel.addEventListener('pointerenter', (event) => { if (event.pointerType === 'mouse') hovered = true; });
  toolsCarousel.addEventListener('pointerleave', (event) => { if (event.pointerType === 'mouse') hovered = false; });
  toolsTrack.addEventListener('wheel', hold, { passive: true });
  toolsTrack.addEventListener('scroll', updateControls, { passive: true });
  toolsTrack.addEventListener('keydown', (event) => {
    const amounts = { ArrowLeft: -step(), ArrowRight: step(), Home: -end(), End: end() };
    if (!(event.key in amounts)) return;
    event.preventDefault();
    move(amounts[event.key]);
  });
  toolsTrack.addEventListener('dragstart', (event) => event.preventDefault());
  toolsTrack.addEventListener('pointerdown', (event) => {
    if (!event.isPrimary || event.button !== 0) return;
    hold();
    interacting = true;
    dragged = false;
    if (event.pointerType === 'mouse') drag = { id: event.pointerId, x: event.clientX, left: toolsTrack.scrollLeft };
  });
  toolsTrack.addEventListener('pointermove', (event) => {
    if (!drag || drag.id !== event.pointerId) return;
    const distance = event.clientX - drag.x;
    if (!dragged && Math.abs(distance) < 6) return;
    dragged = true;
    toolsTrack.classList.add('is-dragging');
    toolsTrack.setPointerCapture(event.pointerId);
    toolsTrack.scrollLeft = drag.left - distance;
  });
  function release(event) {
    if (!interacting) return;
    if (toolsTrack.hasPointerCapture(event.pointerId)) toolsTrack.releasePointerCapture(event.pointerId);
    drag = null;
    interacting = false;
    toolsTrack.classList.remove('is-dragging');
    hold();
  }
  window.addEventListener('pointerup', release);
  window.addEventListener('pointercancel', release);
  window.addEventListener('blur', () => {
    drag = null;
    interacting = false;
    hovered = false;
    toolsTrack.classList.remove('is-dragging');
    hold();
  });
  toolsTrack.addEventListener('click', (event) => {
    // A drag must never launch the app under the pointer; ordinary clicks still do.
    if (dragged && event.detail > 0) { event.preventDefault(); event.stopPropagation(); }
  }, true);
  new IntersectionObserver(([entry]) => { visible = entry.isIntersecting; }, { threshold: .1 }).observe(toolsTrack);
  new ResizeObserver(() => { position = toolsTrack.scrollLeft; updateControls(); }).observe(toolsTrack);
  function animate(now) {
    const elapsed = Math.min(now - lastFrame, 50);
    lastFrame = now;
    // Keep keyboard navigation still without letting a mouse click/drag leave
    // the carousel permanently paused by its lingering DOM focus.
    const keyboardFocus = toolsCarousel.querySelector(':focus-visible');
    if (visible && !document.hidden && !reducedMotion.matches && !hovered && !interacting &&
        !keyboardFocus && now >= resumeAfter && end() > 1) {
      position = Math.max(0, Math.min(end(), position + direction * elapsed * .026));
      toolsTrack.scrollLeft = position;
      if (position >= end() || position <= 0) {
        direction *= -1;
        resumeAfter = now + 1400;
      }
    } else {
      position = toolsTrack.scrollLeft;
    }
    requestAnimationFrame(animate);
  }
  toolsCarousel.querySelector('.tools-controls').hidden = false;
  updateControls();
  requestAnimationFrame(animate);
}

// A slow light cycles from top to bottom, giving each layer equal time. Hover or keyboard focus holds a layer;
// the complete diagram remains readable even with JavaScript or motion disabled.
const architecture = document.querySelector('.architecture');
if (architecture) {
  const layers = ['applications', 'environment', 'protocol'];
  const parts = architecture.querySelectorAll('[data-layer]');
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
  let visible = false;
  let stage = 0;
  let timer;
  let hoveredLayer = null;

  // Keep the illustration aligned with real HTML labels at every text size.
  // The shapes retain their proportions while the gaps adapt to the labels.
  function alignLayers() {
    const svg = architecture.querySelector('svg');
    const bounds = svg.getBoundingClientRect();
    if (!bounds.width) return;
    const scale = bounds.width / svg.viewBox.baseVal.width;
    const labelsTop = architecture.querySelector('.architecture-labels').getBoundingClientRect().top;
    const original = { applications: 72, environment: 182, protocol: 283 };
    const centers = {};
    for (const button of architecture.querySelectorAll('button[data-layer]')) {
      const name = button.dataset.layer;
      centers[name] = (labelsTop + button.offsetTop + button.offsetHeight / 2 - bounds.top) / scale;
      architecture.querySelector(`.architecture-layer[data-layer="${name}"]`)
        .style.setProperty('--layer-offset', `${centers[name] - original[name]}px`);
    }
    const top = centers.applications + 52;
    const middleTop = centers.environment - 41;
    const middleBottom = centers.environment + 52;
    const bottom = centers.protocol - 40;
    architecture.querySelector('[data-layer-links="upper"]').setAttribute('d',
      `M160 ${top}V${middleTop}`);
    architecture.querySelector('[data-layer-links="lower"]').setAttribute('d',
      `M160 ${middleBottom}V${bottom}`);
  }
  new ResizeObserver(alignLayers).observe(architecture);
  window.addEventListener('subnet:languagechange', alignLayers);

  function highlight(layer) {
    architecture.classList.toggle('has-active', Boolean(layer));
    for (const part of parts) {
      const active = part.dataset.layer === layer;
      part.classList.toggle('is-active', active);
      if (part.matches('button')) part.setAttribute('aria-pressed', String(active));
    }
  }

  function updateLayer() {
    clearTimeout(timer);
    if (!visible || document.hidden) {
      highlight(null);
      return;
    }
    const focusedLayer = architecture.querySelector('button[data-layer]:focus-visible')?.dataset.layer;
    const heldLayer = focusedLayer || hoveredLayer;
    if (heldLayer) {
      highlight(heldLayer);
      return;
    }
    if (reducedMotion.matches) {
      highlight(null);
      return;
    }
    highlight(layers[stage]);
    timer = setTimeout(() => {
      stage = (stage + 1) % layers.length;
      updateLayer();
    }, 4200);
  }

  for (const part of parts) {
    part.addEventListener('pointerenter', (event) => {
      if (event.pointerType !== 'mouse') return;
      hoveredLayer = part.dataset.layer;
      updateLayer();
    });
    part.addEventListener('pointerleave', (event) => {
      if (event.pointerType !== 'mouse') return;
      hoveredLayer = null;
      updateLayer();
    });
    part.addEventListener('click', () => {
      stage = layers.indexOf(part.dataset.layer);
      if (reducedMotion.matches) highlight(part.dataset.layer);
      else updateLayer();
    });
  }
  architecture.addEventListener('focusin', updateLayer);
  architecture.addEventListener('focusout', () => queueMicrotask(updateLayer));
  new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    updateLayer();
  }, { threshold: .25 }).observe(architecture);
  document.addEventListener('visibilitychange', updateLayer);
  reducedMotion.addEventListener('change', updateLayer);
}

function openDialog(dialog) {
  if (!(dialog instanceof HTMLDialogElement) || dialog.open) return;
  dialog.showModal();
}
for (const opener of document.querySelectorAll('[data-dialog]')) {
  opener.addEventListener('click', (event) => {
    event.preventDefault();
    openDialog(document.getElementById(opener.dataset.dialog));
  });
}
for (const dialog of document.querySelectorAll('dialog')) {
  dialog.querySelector('[data-close]').addEventListener('click', () => dialog.close());
  for (const link of dialog.querySelectorAll('[data-close-anchor]')) {
    link.addEventListener('click', () => dialog.close());
  }
  dialog.addEventListener('click', (event) => {
    const box = dialog.getBoundingClientRect();
    if (event.target === dialog && (event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom)) dialog.close();
  });
}
// Informational links can also be opened directly or bookmarked.
function openHashDialog() {
  const name = location.hash.slice(1);
  if (name) openDialog(document.getElementById(name));
}
openHashDialog();
window.addEventListener('hashchange', openHashDialog);

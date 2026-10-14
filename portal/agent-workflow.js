/* Illustrative workflow using local samples: no account access or AI requests. */
(() => {
  'use strict';
  const demo = document.querySelector('[data-agent-workflow]');
  if (!demo) return;
  const { t, text } = window.SubnetI18n;
  const part = (name) => demo.querySelector(`[data-wf="${name}"]`);
  const segments = [...demo.querySelectorAll('.wf-segment')];
  const checks = [...demo.querySelectorAll('[data-wf-done]')];
  const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
  const duration = 24000;
  const previewTime = 17400;
  const states = [
    ['Ühendan sinu pilveruumiga…', 'Valmistan ette kampaania lähtefaili lugemise.', 'Pilveruum ühendatud', 'Sinu ülesanne, sinu pilvefailid'],
    ['Laen lähtefaili pilvest alla…', 'Loen faili creative-brief.md AI-agendi tööruumi.', 'Lähtefail saadaval', 'creative-brief.md → AI-agent'],
    ['Mõistan lähteülesannet…', 'Tõlgendan kampaania sõnumit ja visuaalset suunda.', 'Agent töötab lähteülesandega…', ''],
    ['Loon kampaaniapilti…', 'Pilt valmib AI-agendi tööriistaga.', 'Ootan valmivat pilti…', ''],
    ['Saadan PNG-faili pilve…', 'Salvestan faili sovereign-cloud.png tagasi samasse kausta.', 'Võtan faili vastu…', 'sovereign-cloud.png → subnet.ee'],
    ['Valmis. Ava pilt pilves.', 'Fail on salvestatud ja saad selle oma pilveruumis avada.', 'Pilt salvestatud', 'Tulemus on sinu pilves'],
  ];
  let elapsed = motion.matches ? previewTime : 0;
  let paused = motion.matches;
  let visible = false;
  let frame = 0;
  let previousTime = 0;
  let renderedAt = -Infinity;
  let currentState = -1;
  let manualPreview = null;
  const menus = [...demo.querySelectorAll('.wf-file-menu')];
  const briefBlob = new Blob([part('brief-preview').textContent], { type: 'text/markdown;charset=utf-8' });

  function fileMetadata() {
    const date = new Date('2026-10-14T12:00:00Z').toLocaleDateString(window.SubnetI18n.locale, { dateStyle: 'medium' });
    part('brief-meta').textContent = `${briefBlob.size} B · ${date}`;
    const imageSize = new Intl.NumberFormat(window.SubnetI18n.locale, { maximumFractionDigits: 1 }).format(648784 / 1024);
    part('image-meta').textContent = `${imageSize} KB · ${date}`;
  }

  function closeMenus(except) {
    for (const menu of menus) if (menu !== except) menu.open = false;
  }

  function positionCursor() {
    const cursor = part('cursor');
    if (cursor.hidden) return;
    const cloud = demo.querySelector('.wf-cloud').getBoundingClientRect();
    // Aim at the text itself, including when the filename wraps on a phone.
    const lines = [...part('image-name').getClientRects()];
    const target = lines.reduce((widest, line) => line.width > widest.width ? line : widest, lines[0]);
    if (!target) return;
    const progress = Math.max(0, Math.min(1, (elapsed - 15100) / 1500));
    const remaining = Math.pow(1 - progress, 3);
    const x = target.left - cloud.left + target.width / 2 - 2;
    const y = target.top - cloud.top + target.height / 2 - 2;
    const pressing = (elapsed >= 16800 && elapsed < 16920) || (elapsed >= 17100 && elapsed < 17220);
    cursor.style.left = `${x + Math.min(60, cloud.width - x - 28) * remaining}px`;
    cursor.style.top = `${y + 80 * remaining}px`;
    cursor.style.opacity = String(Math.min(1, progress * 5));
    cursor.style.transform = pressing ? 'scale(.82)' : 'scale(1)';
  }

  function controls() {
    part('toggle').firstElementChild.textContent = paused ? '▶' : 'Ⅱ';
    part('toggle').setAttribute('aria-label', t(paused ? 'Esita animatsioon' : 'Peata animatsioon'));
    part('timeline').setAttribute('aria-valuetext', `${Math.round(elapsed / (duration - 1) * 100)}% · ${t(states[Math.max(0, currentState)][0])}`);
  }

  function render() {
    const state = segments.findIndex((segment) => elapsed < Number(segment.dataset.end));
    if (state !== currentState) {
      currentState = state;
      const [title, description, status, transfer] = states[state];
      text(part('title'), title);
      text(part('description'), description);
      text(part('cloud-status'), status);
      text(part('transfer-text'), transfer);
    }
    part('spinner').hidden = elapsed >= 14500;
    part('brief').hidden = elapsed < 3600 || elapsed >= 8200;
    part('generating').hidden = elapsed < 8200 || elapsed >= 11600;
    part('transfer').hidden = elapsed >= 3600 && elapsed < 11600;
    for (const check of checks) check.classList.toggle('is-done', elapsed >= Number(check.dataset.wfDone));
    part('brief-file').classList.toggle('is-highlighted', elapsed >= 2200 && elapsed < 8200);
    part('image-file').inert = elapsed < 14500;
    part('image-file').setAttribute('aria-hidden', String(elapsed < 14500));
    part('image-file').classList.toggle('is-highlighted', elapsed >= 14500 && elapsed < previewTime);
    part('cursor').hidden = elapsed < 15100 || elapsed >= previewTime || manualPreview !== null;
    positionCursor();
    const previewOpen = manualPreview !== null || elapsed >= previewTime;
    const showingBrief = manualPreview === 'brief';
    part('preview-name').textContent = showingBrief ? 'creative-brief.md' : 'sovereign-cloud.png';
    part('brief-preview').hidden = !showingBrief;
    part('image-preview').hidden = showingBrief;
    // Automatic playback must not hide an element that currently owns focus.
    if (previewOpen && part('files').contains(document.activeElement)) part('close-preview').focus({ preventScroll: true });
    if (!previewOpen && part('preview').contains(document.activeElement)) part('toggle').focus({ preventScroll: true });
    part('preview').hidden = !previewOpen;
    part('files').inert = previewOpen;
    part('files').setAttribute('aria-hidden', String(previewOpen));
    part('timeline').value = String(Math.round(elapsed));
    part('timeline').style.setProperty('--wf-progress', `${elapsed / (duration - 1) * 100}%`);
    controls();
    segments.forEach((segment, index) => {
      const start = Number(segment.dataset.start);
      const end = Number(segment.dataset.end);
      const progress = Math.max(0, Math.min(100, (elapsed - start) / (end - start) * 100));
      segment.style.setProperty('--wf-progress', `${progress}%`);
      if (index === state) segment.setAttribute('aria-current', 'step');
      else segment.removeAttribute('aria-current');
    });
  }

  function tick(now) {
    frame = 0;
    if (paused || !visible || document.hidden) return;
    if (previousTime) elapsed = (elapsed + Math.min(now - previousTime, 100)) % duration;
    previousTime = now;
    if (now - renderedAt >= 80) {
      render();
      renderedAt = now;
    }
    positionCursor();
    if (!paused && visible && !document.hidden) frame = requestAnimationFrame(tick);
  }

  function playback() {
    const playing = !paused && visible && !document.hidden;
    demo.classList.toggle('is-playing', playing);
    if (!playing) {
      cancelAnimationFrame(frame);
      frame = 0;
      previousTime = 0;
    } else if (!frame) {
      previousTime = 0;
      frame = requestAnimationFrame(tick);
    }
    controls();
  }

  function seek(time) {
    paused = true;
    manualPreview = null;
    closeMenus();
    elapsed = Math.max(0, Math.min(duration - 1, time));
    render();
    playback();
  }

  part('toggle').addEventListener('click', () => {
    paused = !paused;
    manualPreview = null;
    closeMenus();
    // A static completed illustration starts from the beginning when played.
    if (!paused && elapsed >= previewTime) elapsed = 0;
    render();
    playback();
  });
  segments.forEach((segment, index) => segment.addEventListener('click', () => {
    seek(index === segments.length - 1 ? previewTime : Number(segment.dataset.start));
  }));
  part('timeline').addEventListener('input', (event) => seek(Number(event.target.value)));
  function openFile(file) {
    if (file === 'image') seek(previewTime);
    else {
      manualPreview = 'brief';
      paused = true;
      closeMenus();
      render();
      playback();
    }
    part('close-preview').focus({ preventScroll: true });
  }
  function closePreview() {
    const file = manualPreview === 'brief' ? 'brief' : 'image';
    seek(file === 'brief' ? Math.min(elapsed, 14500) : 14500);
    part(`${file}-file`).focus({ preventScroll: true });
  }
  for (const file of ['brief', 'image']) {
    const row = part(`${file}-file`);
    // Match the drive: double-click the row, or use Ava / Enter.
    row.addEventListener('dblclick', (event) => {
      if (!event.target.closest('button, details')) openFile(file);
    });
    row.addEventListener('keydown', (event) => {
      if (event.target === row && event.key === 'Enter') {
        event.preventDefault();
        openFile(file);
      }
    });
  }
  for (const button of demo.querySelectorAll('[data-wf-open]')) {
    button.addEventListener('click', () => openFile(button.dataset.wfOpen));
  }
  for (const button of demo.querySelectorAll('[data-wf-download]')) {
    button.addEventListener('click', () => {
      closeMenus();
      const brief = button.dataset.wfDownload === 'brief';
      const link = document.createElement('a');
      link.href = brief ? URL.createObjectURL(briefBlob) : '/assets/landing/sovereign-cloud.png';
      link.download = brief ? 'creative-brief.md' : 'sovereign-cloud.png';
      link.click();
      if (brief) setTimeout(() => URL.revokeObjectURL(link.href), 1000);
      button.closest('.wf-file-menu').querySelector('summary').focus({ preventScroll: true });
    });
  }
  for (const menu of menus) menu.querySelector('summary').addEventListener('click', () => closeMenus(menu));
  document.addEventListener('click', (event) => {
    if (!menus.some((menu) => menu.contains(event.target))) closeMenus();
  });
  demo.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape') return;
    const menu = menus.find((item) => item.open);
    if (menu) { menu.open = false; menu.querySelector('summary').focus(); }
    else if (!part('preview').hidden) closePreview();
  });
  part('close-preview').addEventListener('click', closePreview);
  // Keep any focused row, file action or preview stable for keyboard users.
  part('close-preview').addEventListener('focus', () => { paused = true; playback(); });
  part('files').addEventListener('focusin', () => { paused = true; playback(); });
  window.addEventListener('subnet:languagechange', () => { controls(); fileMetadata(); positionCursor(); });
  new ResizeObserver(positionCursor).observe(part('files'));
  document.addEventListener('visibilitychange', playback);
  motion.addEventListener('change', () => {
    if (motion.matches) seek(previewTime);
  });
  new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    playback();
  }, { threshold: .15 }).observe(demo);
  part('close-preview').hidden = false;
  part('controls').hidden = false;
  fileMetadata();
  render();
  playback();
})();

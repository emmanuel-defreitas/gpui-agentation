(() => {
  const $ = (selector) => document.querySelector(selector);
  const ns = 'http://www.w3.org/2000/svg';
  let toastTimer;
  function node(tag, cls, text) {
    const result = document.createElement(tag);
    if (cls) result.className = cls;
    if (text !== undefined) result.textContent = text;
    return result;
  }
  function icon(name) {
    const svg = document.createElementNS(ns, 'svg');
    svg.setAttribute('aria-hidden', 'true');
    const use = document.createElementNS(ns, 'use');
    use.setAttribute('href', `#i-${name}`);
    svg.append(use);
    return svg;
  }
  function notify(message) {
    clearTimeout(toastTimer);
    $('#toast').textContent = message;
    $('#toast').classList.add('visible');
    toastTimer = setTimeout(() => $('#toast').classList.remove('visible'), 2400);
  }
  async function copy(text, button) {
    if (button.disabled) return;
    try {
      if (navigator.clipboard && window.isSecureContext) await navigator.clipboard.writeText(text);
      else {
        const input = node('textarea');
        input.value = text;
        input.style.cssText = 'position:fixed;left:-9999px;top:0';
        document.body.append(input);
        input.select();
        const ok = document.execCommand('copy');
        input.remove();
        if (!ok) throw new Error('Clipboard unavailable');
      }
      const original = button.cloneNode(true);
      button.replaceChildren(icon('check'), node('span', '', 'Copied!'));
      button.classList.add('copied');
      button.disabled = true;
      setTimeout(() => {
        button.replaceChildren(...original.childNodes);
        button.classList.remove('copied');
        button.disabled = false;
      }, 2000);
      notify('Copied to clipboard');
    } catch { notify('Clipboard unavailable. Select and copy the text manually.'); }
  }
  document.querySelectorAll('.copy-code').forEach((button) => {
    button.addEventListener('click', () => copy(button.closest('.code-block').querySelector('pre').textContent, button));
  });
  const themeButton = $('#theme-toggle');
  let savedTheme;
  try { savedTheme = localStorage.getItem('agentation-site-theme'); } catch { /* Optional storage. */ }
  const systemDark = window.matchMedia('(prefers-color-scheme: dark)');
  function theme(value) {
    document.documentElement.dataset.theme = value;
    themeButton?.setAttribute('aria-label', `Switch to ${value === 'dark' ? 'light' : 'dark'} appearance`);
  }
  theme(savedTheme || (systemDark.matches ? 'dark' : 'light'));
  themeButton?.addEventListener('click', () => {
    savedTheme = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    theme(savedTheme);
    try { localStorage.setItem('agentation-site-theme', savedTheme); } catch { /* Optional storage. */ }
  });
  systemDark.addEventListener('change', (event) => { if (!savedTheme) theme(event.matches ? 'dark' : 'light'); });
  const canvas = $('#demo-canvas');
  if (!canvas) return;
  const layer = $('#drawing-layer');
  const dialog = $('#note-dialog');
  const tools = [...document.querySelectorAll('[data-tool]')];
  let tool = 'inspect', nextId = 2, editing = null, gesture = null, draft = null;
  const seed = () => [{ id: 1, target: 'project-row', label: 'Project row', source: 'src/components/project_row.rs:28:5', text: 'Give the project row a little more breathing room.', x: .72, y: .58, shape: null }];
  let comments = seed();
  function point(event) {
    const r = canvas.getBoundingClientRect();
    return { x: Math.max(0, Math.min(1, (event.clientX - r.left) / r.width)), y: Math.max(0, Math.min(1, (event.clientY - r.top) / r.height)) };
  }
  function setTool(value) {
    tool = value;
    canvas.dataset.tool = value;
    tools.forEach((button) => button.setAttribute('aria-pressed', String(button.dataset.tool === value)));
    $('#source-tooltip').hidden = true;
    $('#demo-instruction').textContent = value === 'inspect' ? 'Select an element on the left to leave a comment.' : `Drag on the preview to draw ${value === 'arrow' ? 'an arrow' : value === 'box' ? 'a box' : 'with the pen'}, then add a comment.`;
  }
  tools.forEach((button) => button.addEventListener('click', () => setTool(button.dataset.tool)));
  function draw(shape, temporary = false) {
    const r = canvas.getBoundingClientRect();
    const xy = (p) => `${p.x * r.width},${p.y * r.height}`;
    const svg = document.createElementNS(ns, shape.type === 'box' ? 'rect' : 'path');
    if (shape.type === 'box') {
      Object.entries({ x: Math.min(shape.from.x, shape.to.x) * r.width, y: Math.min(shape.from.y, shape.to.y) * r.height, width: Math.abs(shape.to.x - shape.from.x) * r.width, height: Math.abs(shape.to.y - shape.from.y) * r.height, rx: 3 }).forEach(([key, value]) => svg.setAttribute(key, value));
    } else if (shape.type === 'pen') {
      svg.setAttribute('d', shape.points.map((p, i) => `${i ? 'L' : 'M'}${xy(p)}`).join(' '));
    } else {
      const a = { x: shape.from.x * r.width, y: shape.from.y * r.height };
      const b = { x: shape.to.x * r.width, y: shape.to.y * r.height };
      const angle = Math.atan2(b.y - a.y, b.x - a.x);
      svg.setAttribute('d', `M${a.x},${a.y} L${b.x},${b.y} M${b.x - 11 * Math.cos(angle - .5)},${b.y - 11 * Math.sin(angle - .5)} L${b.x},${b.y} L${b.x - 11 * Math.cos(angle + .5)},${b.y - 11 * Math.sin(angle + .5)}`);
    }
    Object.entries({ fill: 'none', stroke: '#b77a00', 'stroke-width': 2.5, 'stroke-linecap': 'round', 'stroke-linejoin': 'round' }).forEach(([key, value]) => svg.setAttribute(key, value));
    if (temporary) svg.setAttribute('stroke-dasharray', '5 3');
    layer.append(svg);
  }
  function paint() {
    $('#pin-layer').replaceChildren();
    layer.replaceChildren();
    comments.forEach((comment) => {
      if (comment.shape) draw(comment.shape);
      let position = comment;
      const target = comment.target && canvas.querySelector(`[data-target="${comment.target}"]`);
      if (target) {
        const r = target.getBoundingClientRect(), parent = canvas.getBoundingClientRect();
        position = { x: (r.right - parent.left - 8) / parent.width, y: (r.top - parent.top + 7) / parent.height };
      }
      const pin = node('button', 'pin', String(comment.id));
      pin.style.left = `${position.x * 100}%`;
      pin.style.top = `${position.y * 100}%`;
      pin.setAttribute('aria-label', `Edit comment ${comment.id}: ${comment.label}`);
      pin.addEventListener('click', (event) => { event.stopPropagation(); openNote(comment); });
      $('#pin-layer').append(pin);
    });
    if (draft?.shape) draw(draft.shape, true);
    if (gesture) draw(gesture.shape, true);
  }
  function render() {
    const list = $('#comment-list');
    list.replaceChildren();
    $('#comment-count').textContent = comments.length;
    if (!comments.length) list.append(node('p', 'empty-comments', 'No comments yet. Select a tool and mark a place.'));
    comments.forEach((comment) => {
      const entry = node('article', 'comment-entry'), top = node('div', 'comment-entry-top');
      top.append(node('span', 'pin', String(comment.id)));
      const body = node('div', 'comment-entry-body'), edit = node('button');
      edit.setAttribute('aria-label', `Edit comment ${comment.id}`);
      edit.append(node('strong', '', comment.label), node('p', '', comment.text), node('div', 'comment-source', comment.source || `drawing · ${comment.shape?.type || 'pin'}`));
      edit.addEventListener('click', () => openNote(comment));
      body.append(edit);
      const remove = node('button', 'comment-delete');
      remove.setAttribute('aria-label', `Delete comment ${comment.id}`);
      remove.append(icon('trash'));
      remove.addEventListener('click', () => { comments = comments.filter((item) => item.id !== comment.id); render(); notify(`Comment ${comment.id} deleted`); });
      top.append(body, remove);
      entry.append(top);
      list.append(entry);
    });
    paint();
  }
  function openNote(comment) {
    if (comments.length >= 20 && !comment.id) { notify('Preview limit reached. Reset to start again.'); return; }
    editing = comment.id || null;
    draft = comment;
    $('#note-title').textContent = editing ? `Comment ${comment.id}` : 'New annotation';
    $('#note-target').textContent = comment.source ? `${comment.label} · ${comment.source}` : comment.label;
    $('#note-text').value = comment.text || '';
    $('#source-tooltip').hidden = true;
    dialog.showModal();
    $('#note-text').focus();
  }
  $('#note-form').addEventListener('submit', (event) => {
    if (event.submitter?.value !== 'save') return;
    event.preventDefault();
    const text = $('#note-text').value.trim();
    if (!text) { $('#note-text').focus(); return; }
    if (editing) comments = comments.map((item) => item.id === editing ? { ...item, text } : item);
    else comments.push({ ...draft, id: nextId++, text });
    draft = null;
    dialog.close();
    render();
    notify('Comment saved. Copy the feedback to share it.');
  });
  dialog.addEventListener('close', () => { draft = null; editing = null; paint(); });
  dialog.addEventListener('click', (event) => {
    if (event.target !== dialog) return;
    const r = dialog.getBoundingClientRect();
    if (event.clientX < r.left || event.clientX > r.right || event.clientY < r.top || event.clientY > r.bottom) dialog.close();
  });
  canvas.addEventListener('click', (event) => {
    if (tool !== 'inspect' || event.target.closest('.pin')) return;
    const target = event.target.closest('[data-target]');
    if (target) openNote({ target: target.dataset.target, label: target.dataset.label, source: target.dataset.source, ...point(event), shape: null });
  });
  canvas.addEventListener('keydown', (event) => {
    if (['Enter', ' '].includes(event.key) && event.target.matches('[data-target]')) {
      event.preventDefault();
      const t = event.target;
      openNote({ target: t.dataset.target, label: t.dataset.label, source: t.dataset.source, x: .5, y: .5, shape: null });
    }
  });
  canvas.addEventListener('pointerdown', (event) => {
    if (tool === 'inspect' || event.button !== 0 || event.target.closest('.pin')) return;
    event.preventDefault();
    const p = point(event);
    gesture = { pointerId: event.pointerId, shape: { type: tool, from: p, to: p, points: [p] } };
    canvas.setPointerCapture(event.pointerId);
  });
  canvas.addEventListener('pointermove', (event) => {
    if (gesture && gesture.pointerId === event.pointerId) {
      const p = point(event);
      gesture.shape.to = p;
      if (gesture.shape.points.length < 2000) gesture.shape.points.push(p);
      paint();
    } else if (tool === 'inspect') {
      const target = event.target.closest('[data-target]');
      $('#source-tooltip').hidden = !target;
      if (target) $('#source-tooltip').textContent = `${target.dataset.label} · ${target.dataset.source}`;
    }
  });
  canvas.addEventListener('pointerleave', () => { $('#source-tooltip').hidden = true; });
  canvas.addEventListener('pointerup', (event) => {
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    const shape = gesture.shape;
    gesture = null;
    if (canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
    const r = canvas.getBoundingClientRect();
    if (Math.hypot((shape.to.x - shape.from.x) * r.width, (shape.to.y - shape.from.y) * r.height) < 6 && shape.points.length < 5) { paint(); return; }
    openNote({ label: `${shape.type[0].toUpperCase()}${shape.type.slice(1)} annotation`, x: (shape.from.x + shape.to.x) / 2, y: (shape.from.y + shape.to.y) / 2, shape });
  });
  function cancelGesture() { gesture = null; paint(); }
  canvas.addEventListener('pointercancel', cancelGesture);
  document.addEventListener('keydown', (event) => { if (event.key === 'Escape' && gesture) cancelGesture(); });
  new ResizeObserver(paint).observe(canvas);
  $('#reset-demo').addEventListener('click', () => { comments = seed(); nextId = 2; draft = null; gesture = null; setTool('inspect'); render(); notify('Preview reset'); });
  $('#comments-tool').addEventListener('click', () => {
    const panel = $('#comment-panel');
    panel.hidden = !panel.hidden;
    $('.workbench').classList.toggle('comments-hidden', panel.hidden);
    $('#comments-tool').setAttribute('aria-expanded', String(!panel.hidden));
  });
  $('#copy-feedback').addEventListener('click', () => {
    if (!comments.length) { notify('Add a comment before copying feedback.'); return; }
    const markdown = `# Fieldwork — annotation feedback\n\nBrowser preview · synthetic app · gpui://fieldwork/overview\n\n${comments.map((item) => `## ${item.id}. ${item.label}\n\n${item.text}\n\n${item.source ? `Source: \`${item.source}\`` : `Shape: \`${JSON.stringify(item.shape)}\``}`).join('\n\n')}\n`;
    copy(markdown, $('#copy-feedback'));
  });
  const links = [...document.querySelectorAll('.docs-nav a[href^="#"]')];
  const observer = new IntersectionObserver((entries) => {
    const current = entries.find((entry) => entry.isIntersecting);
    if (!current) return;
    links.forEach((link) => {
      const active = link.hash === `#${current.target.id}`;
      link.classList.toggle('active', active);
      if (active) link.setAttribute('aria-current', 'location'); else link.removeAttribute('aria-current');
    });
  }, { rootMargin: '-8% 0px -65% 0px' });
  document.querySelectorAll('.doc-section').forEach((section) => observer.observe(section));
  setTool('inspect');
  render();
})();

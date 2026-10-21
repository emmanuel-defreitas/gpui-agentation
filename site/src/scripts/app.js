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
})();

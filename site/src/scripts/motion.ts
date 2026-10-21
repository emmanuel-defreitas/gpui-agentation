import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(ScrollTrigger);
const film = document.querySelector<HTMLImageElement>('#demo-film');
const toggle = document.querySelector<HTMLButtonElement>('#film-toggle');
const reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
let playing = !reduced.matches;
const loops: {timeline: gsap.core.Timeline; visible: boolean}[] = [];
function syncLoops() {
  loops.forEach(loop => {
    if (!playing) loop.timeline.progress(1).pause();
    else if (loop.visible) loop.timeline.play();
    else loop.timeline.pause();
  });
}

function updateFilm() {
  if (!film || !toggle) return;
  const src = playing ? film.dataset.gif : film.dataset.poster;
  if (src && film.getAttribute('src') !== src) film.src = src;
  toggle.hidden = false;
  toggle.setAttribute('aria-label', playing ? 'Pause animations' : 'Play animations');
  toggle.querySelector('use')?.setAttribute('href', playing ? '#i-pause' : '#i-play');
  const label = toggle.querySelector('span');
  if (label) label.textContent = playing ? 'Pause' : 'Play';
}
toggle?.addEventListener('click', () => { playing = !playing; updateFilm(); syncLoops(); });
reduced.addEventListener('change', () => { playing = !reduced.matches; updateFilm(); syncLoops(); });
updateFilm();

const contexts = gsap.matchMedia();
contexts.add('(prefers-reduced-motion: no-preference)', () => {
  gsap.from('.feature', {
    y: 16, autoAlpha: 0, duration: .65, stagger: .12, ease: 'power3.out',
    scrollTrigger: {trigger: '.feature-grid', start: 'top 92%', once: true},
  });
  document.querySelectorAll<HTMLElement>('[data-motion]').forEach((card) => {
    const kind = card.dataset.motion;
    const timeline = gsap.timeline({repeat: -1, repeatDelay: 1.8, paused: true, defaults:{ease:'power2.inOut'}});
    const find = (selector: string) => card.querySelector(selector);
    if (kind === 'inspect') {
      timeline.fromTo(find('.inspect-cursor'), {x:28,y:20}, {x:0,y:0,duration:.8})
        .fromTo(find('.inspect-outline'), {opacity:.12}, {opacity:1,duration:.45}, '-=.15')
        .fromTo(find('.inspect-source'), {opacity:0,y:5}, {opacity:1,y:0,duration:.45})
        .to({}, {duration:1.5});
    } else if (kind === 'draw') {
      ['.draw-box','.draw-arrow','.draw-pen'].forEach((selector) => {
        timeline.fromTo(find(selector), {strokeDasharray:1,strokeDashoffset:1}, {strokeDashoffset:0,duration:.75});
      });
      timeline.to({}, {duration:1.1});
    } else {
      timeline.fromTo(find('.handoff-dot'), {x:0,opacity:0}, {opacity:1,duration:.2})
        .to(find('.handoff-dot'), {x:24,duration:.8})
        .fromTo(find('.handoff-status'), {opacity:0,y:4}, {opacity:1,y:0,duration:.45})
        .to(find('.handoff-dot'), {opacity:0,duration:.2})
        .to({}, {duration:1.6});
    }
    const loop = {timeline, visible: false};
    loops.push(loop);
    ScrollTrigger.create({trigger:card,start:'top bottom',end:'bottom top',onToggle:state=>{loop.visible=state.isActive;syncLoops();}});
  });
  return () => { loops.length = 0; };
});

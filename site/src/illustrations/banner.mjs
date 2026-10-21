// Original vector artwork, inspired by the Agentation reference supplied by the user.
export const WIDTH = 1120;
export const HEIGHT = 650;
export const DURATION = 21;
const STEP = 4.7;
const notes = [
  {title:'Overview card',comment:'Give this card more room.',target:[286,246],popup:[196,242],pin:[216,194],region:[138,190,294,120]},
  {title:'Recent activity',comment:'Increase the space between rows.',target:[594,420],popup:[458,214],pin:[464,326],region:[138,378,914,88]},
  {title:'Export button',comment:'Make this the primary action.',target:[1008,148],popup:[644,190],pin:[1020,148],region:[964,118,88,60]},
  {title:'Sidebar icons',comment:'Add hover states to these icons.',target:[66,230],popup:[64,174],pin:[66,196],region:[48,186,42,136]},
];
const clamp = (x) => Math.max(0,Math.min(1,x));
const ease = (x) => {x=clamp(x);return x<.5?2*x*x:1-((-2*x+2)**2)/2;};
const mix = (a,b,t) => a+(b-a)*t;
const escape = (text) => text.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;');
function wrapped(text) {
  const words=text.split(' '),lines=[''];
  for(const word of words) {
    if(lines.at(-1).length+word.length>29)lines.push(word);
    else lines[lines.length-1]+=(lines.at(-1)?' ':'')+word;
  }
  return lines;
}
export function bannerState(time) {
  const t = Math.max(0,Math.min(DURATION-.001,time));
  const stage=Math.min(3,Math.floor(t/STEP));
  const local=t-stage*STEP;
  const note=notes[stage];
  const previous=stage?notes[stage-1]:null;
  const from=previous?[previous.popup[0]+330,previous.popup[1]+188]:[938,560];
  const enter=ease(local/.85);
  let cursor=[mix(from[0],note.target[0],enter),mix(from[1],note.target[1],enter)];
  const toSave=ease((local-3.45)/.42);
  if(local>3.45)cursor=[mix(note.target[0],note.popup[0]+330,toSave),mix(note.target[1],note.popup[1]+188,toSave)];
  const saved = local>4.03;
  const progress=clamp((local-1.38)/1.85);
  const text=note.comment.slice(0,Math.floor(note.comment.length*progress));
  const popupAlpha=clamp((local-1.02)/.22)*(1-clamp((local-3.95)/.18));
  const reset=clamp((t-19.6)/1.4);
  return {stage,local,note,cursor,text,progress,saved,popupAlpha,reset,
    selectAlpha:clamp((local-.72)/.18)*(1-clamp((local-4.03)/.3)),
    pinAlpha:notes.map((_,i)=>i<stage?1:i===stage&&saved?1:0),
    click:clamp(1-Math.abs(local-3.92)/.15),
  };
}
const rect = (x,y,w,h,fill,rx=8,other='') => `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${rx}" fill="${fill}" ${other}/>`;
const line = (x1,y1,x2,y2,color='#e0e5e9') => `<path d="M${x1} ${y1}H${x2}" stroke="${color}" stroke-width="1.5"/>`;
function toolbar() {
  const icons=[
    '<path d="M-5-10V10M5-10V10"/>',
    '<path d="M-13 0q13-19 26 0-13 19-26 0Z"/><circle r="4.5"/>',
    '<rect x="-8" y="-5" width="15" height="17" rx="3"/><path d="M-3-10H9a3 3 0 0 1 3 3V5"/>',
    '<path d="M-9-7H9M-6-7l1 18H5L6-7M-3-7v-4H3v4M-2-3V7M2-3V7"/>',
    '<path d="m0-12 3 3 5-1 1 5 3 3-3 3-1 5-5-1-3 3-3-3-5 1-1-5-3-3 3-3 1-5 5 1Z"/><circle r="4"/>',
  ];
  return `<g filter="url(#toolbar-shadow)">${rect(666,532,408,72,'#1b2227',36)}</g><g stroke="#cbd2d7" stroke-width="2.3" fill="none" stroke-linecap="round" stroke-linejoin="round">${icons.map((icon,i)=>`<g transform="translate(${704+i*62} 568)">${icon}</g>`).join('')}<path d="M990 556V580" stroke="#495056"/><path d="m1018 558 20 20m-20 0 20-20"/></g>`;
}
export function renderBanner(time=17) {
  const s=bannerState(time),n=s.note;
  const textLines=wrapped(s.text);
  const typing=textLines.map((text,i)=>`<tspan x="42" dy="${i?27:0}">${escape(text)}</tspan>`).join('');
  const caret=s.progress<1?(Math.sin(time*9)>-.2?1:0):0;
  const fields=[142,452,762].map((x,i)=>`<g filter="url(#card-shadow)">${rect(x,194,286,112,'#f8fafc',18)}</g>${rect(x+24,218,110-i*18,10,'#dfe4e9',4)}${rect(x+24,244,132-i*15,36,'#d7dde3',6)}`).join('');
  const rows=[414,502,582].map(y=>`${rect(170,y,140-(y%3)*15,14,'#e0e5eb',4)}${rect(350,y,100,14,'#e0e5eb',4)}${rect(490,y-12,94,36,'#eaf0f5',18)}`).join('');
  const pin=(i)=>`<g opacity="${s.pinAlpha[i]}"><circle cx="${notes[i].pin[0]}" cy="${notes[i].pin[1]}" r="22" fill="#f4b942" filter="url(#pin-shadow)"/><text x="${notes[i].pin[0]}" y="${notes[i].pin[1]+7}" text-anchor="middle" font-size="22" font-weight="600" fill="#523600">${i+1}</text></g>`;
  const region=n.region;
  const active=stageColor(s.stage);
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}" fill="none">
  <defs><filter id="window-shadow" x="-10%" y="-10%" width="120%" height="125%"><feDropShadow dx="0" dy="6" stdDeviation="10" flood-color="#344454" flood-opacity=".09"/></filter><filter id="card-shadow" x="-10%" y="-10%" width="120%" height="130%"><feDropShadow dx="0" dy="2" stdDeviation="2" flood-color="#344454" flood-opacity=".06"/></filter><filter id="note-shadow" x="-35%" y="-40%" width="170%" height="200%"><feDropShadow dx="0" dy="12" stdDeviation="18" flood-color="#1b252e" flood-opacity=".25"/></filter><filter id="pin-shadow" x="-50%" y="-50%" width="200%" height="210%"><feDropShadow dx="0" dy="3" stdDeviation="4" flood-color="#182a40" flood-opacity=".18"/></filter><filter id="toolbar-shadow" x="-20%" y="-70%" width="140%" height="250%"><feDropShadow dx="0" dy="6" stdDeviation="10" flood-color="#152028" flood-opacity=".28"/></filter><clipPath id="window-clip">${rect(24,12,1072,620,'white',26)}</clipPath></defs>
  <style>text{font-family:Inter,Arial,sans-serif}text{stroke:none}</style>
  ${rect(0,0,WIDTH,HEIGHT,'#f3f7fa',0)}
  <g filter="url(#window-shadow)">${rect(24,12,1072,620,'#edf1f5',26)}</g>
  <g clip-path="url(#window-clip)">
  ${rect(24,12,1072,80,'#f8fafc',0)}${rect(150,33,918,40,'#eaf0f5',13)}
  <circle cx="56" cy="52" r="8" fill="#ff605c"/><circle cx="84" cy="52" r="8" fill="#ffbd44"/><circle cx="112" cy="52" r="8" fill="#00ca4e"/>
  <text x="170" y="60" fill="#919ba5" font-size="21">Fieldwork — development build</text>
  ${line(24,92,1096,92)}${rect(24,94,88,538,'#f7f9fc',0)}<path d="M112 94V632" stroke="#e0e5ea" stroke-width="1.5"/>
  ${rect(48,118,40,40,'#dbe2e9',10)}${[188,238,288,338].map(y=>rect(52,y,32,32,'#dbe1e7',8)).join('')}<circle cx="68" cy="594" r="16" fill="#e0e6ec"/>
  ${rect(142,130,36,36,'#d1d9df',10)}<text x="194" y="157" font-size="22" font-weight="500" fill="#747e87">Fieldwork dashboard</text>
  ${rect(968,122,80,54,'#d7dde3',11)}${fields}
  <g filter="url(#card-shadow)">${rect(142,326,906,288,'#f8fafc',18)}</g>
  ${rect(170,346,116,12,'#dce2e8',4)}${rect(350,346,98,12,'#dce2e8',4)}${rect(490,346,102,12,'#dce2e8',4)}
  ${[380,468,550].map(y=>line(142,y,1048,y)).join('')}${rows}
  <g opacity="${1-s.reset}">
  ${s.pinAlpha.map((_,i)=>pin(i)).join('')}
  <rect x="${region[0]}" y="${region[1]}" width="${region[2]}" height="${region[3]}" rx="12" fill="${active}" fill-opacity=".03" stroke="${active}" stroke-width="3" ${s.stage===3?'stroke-dasharray="6 5"':''} opacity="${s.selectAlpha}" style="stroke:${active}"/>
  <g transform="translate(${n.popup[0]} ${n.popup[1]+(1-s.popupAlpha)*6})" opacity="${s.popupAlpha}">
    <g filter="url(#note-shadow)">${rect(0,0,400,228,'#ffdda0',30)}</g>
    <text x="24" y="39" fill="#786237" font-size="20">${n.title}</text>
    ${rect(24,57,352,80,'#fff4db',12,'style="stroke:#c58a15;stroke-width:2"')}
    <text x="42" y="92" fill="#392700" font-size="21">${typing}<tspan opacity="${caret}" fill="#523600">|</tspan></text>
    <text x="258" y="195" fill="#786237" font-size="20" font-weight="500" text-anchor="end">Cancel</text>
    ${rect(286,166,88,44,s.click>.25?'#10a95a':'#16c76d',22)}
    <text x="330" y="195" fill="white" font-size="21" font-weight="600" text-anchor="middle">Add</text>
  </g>
  <g transform="translate(${s.cursor[0]} ${s.cursor[1]})" opacity="${1-s.reset}" stroke-linecap="round" stroke-linejoin="round">
  ${s.local<3.45?'<path d="M-16 0H16M0-16V16" stroke="#f8fafc" stroke-width="4"/><path d="M-16 0H16M0-16V16" stroke="#18232c" stroke-width="1.8"/>':'<path d="M0 0V24L6 18l6 12 5-3-6-11h11Z" fill="#18232c" stroke="white" stroke-width="2"/>'}
  </g></g>
  ${toolbar()}
  </g>
  ${rect(24,12,1072,620,'none',26,'style="stroke:#dbe3ea;stroke-width:1.5"')}
  </svg>`;
}
function stageColor(stage){return stage===3?'#18bc65':'#5b91f8';}

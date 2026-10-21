import { Resvg } from '@resvg/resvg-js';
import { mkdirSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { renderBanner, DURATION } from '../src/illustrations/banner.mjs';

const site = fileURLToPath(new URL('../', import.meta.url));
const output = join(site,'public/assets/motion');
const frames = mkdtempSync(join(tmpdir(),'agentation-banner-'));
const fps = 12;
mkdirSync(output,{recursive:true});
const fonts=['Inter-Regular.ttf','Inter-SemiBold.ttf'].map(name=>join(site,'src/assets/fonts',name));
const render = svg => new Resvg(svg,{font:{fontFiles:fonts,loadSystemFonts:false,defaultFontFamily:'Inter'}}).render().asPng();
try {
  for(let frame=0;frame<DURATION*fps;frame++) {
    writeFileSync(join(frames,`${String(frame).padStart(4,'0')}.png`),render(renderBanner(frame/fps)));
  }
  writeFileSync(join(output,'dashboard-demo-poster.svg'),renderBanner(17.4));
  const args=['-hide_banner','-loglevel','error','-y','-framerate',String(fps),'-i',join(frames,'%04d.png'),'-vf','split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3','-loop','0',join(output,'dashboard-demo.gif')];
  const result=spawnSync('ffmpeg',args,{stdio:'inherit'});
  if(result.status!==0)throw new Error('GIF encoding failed');
  console.log(`Rendered ${DURATION}s vector banner at ${fps} fps.`);
} finally { rmSync(frames,{recursive:true,force:true}); }

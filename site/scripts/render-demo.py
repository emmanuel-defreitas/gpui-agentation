"""Reproduce the website GIF/poster from the original generated HeyGen MP4.

Requires FFmpeg. Usage: python3 scripts/render-demo.py /path/to/source.mp4
The authored, published outputs are committed; this is not part of site builds.
"""
import argparse
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path, nargs="?", default=Path(__file__).resolve().parents[1] / "media-source/annotation-demo.mp4")
args = parser.parse_args()
output = Path(__file__).resolve().parents[1] / "public/assets/motion"
output.mkdir(parents=True, exist_ok=True)
# Remove color casts from neutral pixels, retaining the intentional amber accent.
rgb = [f"{c}(X,Y)" for c in "rgb"]
neutral = f"lt(max({rgb[0]},max({rgb[1]},{rgb[2]}))-min({rgb[0]},min({rgb[1]},{rgb[2]})),24)"
gray = f"({' + '.join(rgb)})/3"
channels = ":".join(f"{c}='if({neutral},{gray},{c}(X,Y))'" for c in "rgb")
base = f"crop=920:520:232:128,format=rgb24,geq={channels}"
common = ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y"]
subprocess.run(common + ["-i", str(args.source), "-an", "-vf", base + ",fps=12,scale=1104:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3", "-loop", "0", str(output / "annotation-demo.gif")], check=True)
subprocess.run(common + ["-ss", "2", "-i", str(args.source), "-vf", base + ",scale=1104:-1:flags=lanczos", "-frames:v", "1", "-update", "1", str(output / "annotation-demo-poster.png")], check=True)

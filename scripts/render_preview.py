#!/usr/bin/env python3
"""Render TestBackend cell captures as PNGs. Development-only: requires Pillow."""
import argparse
import json
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=Path, nargs="?", default=Path("target/previews"))
parser.add_argument("--font", default="/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf")
args = parser.parse_args()
font = ImageFont.truetype(args.font, 16)
bold_path = str(Path(args.font).with_name(Path(args.font).stem + "-Bold.ttf"))
bold = ImageFont.truetype(bold_path if Path(bold_path).exists() else args.font, 16)
for path in sorted(args.directory.glob("*.json")):
    data = json.loads(path.read_text())
    image = Image.new("RGB", (data["width"] * 10 + 32, data["height"] * 23 + 32), data["cells"][0][0]["bg"])
    draw = ImageDraw.Draw(image)
    for y, row in enumerate(data["cells"]):
        for x, cell in enumerate(row):
            left, top = 16 + x * 10, 16 + y * 23
            draw.rectangle((left, top, left + 9, top + 22), fill=cell["bg"])
            # Terminals tile block glyphs to cell edges; font rasterizers add gaps.
            block = {"█": (0, 0, 9, 22), "▀": (0, 0, 9, 10),
                     "▄": (0, 11, 9, 22), "▎": (0, 0, 1, 22)}.get(cell["text"])
            if block:
                x0, y0, x1, y1 = block
                draw.rectangle((left + x0, top + y0, left + x1, top + y1), fill=cell["fg"])
            else:
                draw.text((left, top + 1), cell["text"], font=bold if cell["bold"] else font, fill=cell["fg"])
            if cell["underline"]:
                draw.line((left, top + 20, left + 9, top + 20), fill=cell["fg"])
    image.save(path.with_suffix(".png"))
print(f"Rendered terminal captures in {args.directory}")

#!/usr/bin/env python3
"""Compares two renders: mean |difference| (0-255), share of pixels off by more
than 8, and a sheet (a | b | difference x8) next to the first image.
   tools/compare.py a.png b.png [sheet.png]"""
import sys
from PIL import Image, ImageChops
a = Image.open(sys.argv[1]).convert("RGB")
b = Image.open(sys.argv[2]).convert("RGB")
d = ImageChops.difference(a, b)
px = list(d.getdata())
mean = sum(sum(p) for p in px) / (3 * len(px))
off = sum(1 for p in px if max(p) > 8) / len(px)
print(f"mean |diff| {mean:.2f}, pixels off by >8: {off*100:.2f}%")
if len(sys.argv) > 3:
    w, h = a.size
    sheet = Image.new("RGB", (w * 3, h))
    sheet.paste(a, (0, 0))
    sheet.paste(b, (w, 0))
    sheet.paste(d.point(lambda v: min(255, v * 8)), (2 * w, 0))
    sheet.save(sys.argv[3])

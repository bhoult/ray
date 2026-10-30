"""The README's banner: the ray-traced banner scene (assets/banner.json,
rendered 1600x500) with the name and a line about it set on the right, in
Lato (fonts-lato).
    python3 tools/logo.py banner-render.png docs/images/logo.jpg
"""
import sys
from PIL import Image, ImageDraw, ImageFilter, ImageFont

LATO = '/usr/share/fonts/truetype/lato/Lato-{}.ttf'
src, out = sys.argv[1], sys.argv[2]
bg = Image.open(src).convert('RGBA')
W, H = bg.size
# a soft shade behind the words, so they read on the bright sky
shade = Image.new('L', (W, H), 0)
sd = ImageDraw.Draw(shade)
for x in range(W):
    k = max(0.0, min(1.0, (x - 0.38 * W) / (0.25 * W)))
    sd.line([(x, 0), (x, H)], fill=int(95 * k))
bg = Image.composite(Image.new('RGBA', (W, H), (25, 12, 8, 255)), bg, shade)
big = ImageFont.truetype(LATO.format('Black'), int(0.42 * H))
x0 = int(0.47 * W)
lines = ['mesh-free ray tracing of pure maths', 'SDFs · fractals · NURBS · CPU and GPU']
# (the lines as big as fit)
size = int(0.088 * H)
while True:
    small = ImageFont.truetype(LATO.format('Medium'), size)
    if max(small.getlength(t) for t in lines) <= W - x0 - 0.03 * W:
        break
    size -= 1
top = int(0.148 * H)
tb = big.getbbox('ray')
y_title = top - tb[1]
ys = [top + (tb[3] - tb[1]) + int(0.072 * H)]
ys.append(ys[0] + int(size * 1.35))
# the words' shadow, blurred
sh = Image.new('RGBA', (W, H), (0, 0, 0, 0))
d = ImageDraw.Draw(sh)
d.text((x0 + 4, y_title + 5), 'ray', font=big, fill=(0, 0, 0, 170))
for t, y in zip(lines, ys):
    d.text((x0 + 8, y + 4), t, font=small, fill=(0, 0, 0, 200))
bg = Image.alpha_composite(bg, sh.filter(ImageFilter.GaussianBlur(7)))
d = ImageDraw.Draw(bg)
d.text((x0, y_title), 'ray', font=big, fill=(255, 247, 236, 255))
for t, y in zip(lines, ys):
    d.text((x0 + 6, y), t, font=small, fill=(255, 228, 194, 255))
bg.convert('RGB').save(out, quality=90)

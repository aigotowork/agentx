"""Lay out the verified browser captures as a single review sheet."""
from pathlib import Path
import sys
from PIL import Image, ImageDraw, ImageFont

source, target = map(Path, sys.argv[1:])
sheet = Image.new("RGB", (1600, 1252), "#eeece6")
draw = ImageDraw.Draw(sheet)
fonts = Path("/usr/share/fonts/opentype/noto")


def label(x, y, text, size, color, *, bold=False, serif=False):
    family = "Serif" if serif else "Sans"
    weight = "Bold" if bold else "Regular"
    font = ImageFont.truetype(str(fonts / f"Noto{family}CJK-{weight}.ttc"), size, index=2)
    draw.text((x, y), text, font=font, fill=color, anchor="lt")


def capture(name, x, y, w, h):
    image = Image.open(source / (name + ".png")).convert("RGB")
    image.thumbnail((w, h), Image.Resampling.LANCZOS)
    sheet.paste(image, (x, y))
    draw.rectangle((x, y, x + image.width - 1, y + image.height - 1), outline="#948e7f")


label(44, 36, "AGENTX SPACE    /    DESIGN CANVAS", 16, "#494e4a", bold=True)
label(44, 66, "同一个工坊，两种会话体验", 39, "#26332f", bold=True, serif=True)
label(1300, 75, "方案 01   /   02", 22, "#5e5b50")

for style, x, title, number, palette, ink, muted in [
    ("folio", 44, "工坊信笺", "01", ("#fcf6e7", "#e8ead7", "#3d624d", "#874b33"), "#263a31", "#5c6156"),
    ("signal", 812, "夜班联络台", "02", ("#152432", "#254054", "#83e2cb", "#ffc7b9"), "#20354c", "#515b66"),
]:
    draw.line((x, 137, x + 744, 137), fill=ink, width=2)
    label(x, 158, number, 20, muted, bold=True)
    label(x + 46, 151, title, 30, ink, bold=True, serif=(style == "folio"))
    label(x + 490, 163, style.upper(), 15, muted, bold=True)
    for index, color in enumerate(palette):
        left = x + 594 + index * 38
        draw.ellipse((left, 159, left + 25, 184), fill=color, outline="#867d70")
    capture(style + "-reader", x, 206, 744, 465)
    label(x, 701, "新建 Agent", 17, muted, bold=True)
    label(x + 484, 701, "手机会话", 17, muted, bold=True)
    capture(style + "-modal-detail", x, 732, 416, 324)
    capture(style + "-confirmation-detail", x, 1073, 416, 141)
    capture(style + "-mobile", x + 484, 732, 248, 480)

sheet.save(target, optimize=True)

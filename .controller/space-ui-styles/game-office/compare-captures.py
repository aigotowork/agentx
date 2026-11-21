"""Collect the runner's captures into three labeled sheets, preserving the originals."""
from pathlib import Path
import sys

from PIL import Image, ImageDraw

folder = Path(sys.argv[1])
styles = ("inkpost", "windward", "jadecourt")
width = 560
groups = {
    "desktop-comparison": ("{style}-1440-room", "{style}-1440-reader"),
    "design-comparison": ("design-{style}-room",),
    "lantern-comparison": ("{style}-lamp-on", "{style}-lamp-off"),
}
for sheet_name, patterns in groups.items():
    rows = []
    for pattern in patterns:
        row = []
        for style in styles:
            name = pattern.format(style=style)
            with Image.open(folder / (name + ".png")) as source:
                height = round(width * source.height / source.width)
                row.append((name, source.convert("RGB").resize((width, height), Image.Resampling.LANCZOS)))
        rows.append((max(image.height for _, image in row) + 25, row))
    sheet = Image.new("RGB", (width * len(styles), sum(height for height, _ in rows)), "#20262c")
    draw = ImageDraw.Draw(sheet)
    y = 0
    for height, row in rows:
        for col, (name, image) in enumerate(row):
            x = col * width
            draw.text((x + 10, y + 6), name, fill="white")
            sheet.paste(image, (x, y + 25))
        y += height
    sheet.save(folder / (sheet_name + ".png"))

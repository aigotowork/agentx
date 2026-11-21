"""Compose the three published game-office views from unmodified browser captures."""
from pathlib import Path
import sys
from PIL import Image, ImageDraw, ImageFont

source, target = map(Path, sys.argv[1:])
target.mkdir(parents=True, exist_ok=True)
worlds = (("inkpost", "烛影邮局", "INKPOST", "#472d38", "#da9a5e"),
          ("windward", "风屿驿站", "WINDWARD", "#294f59", "#91b8af"),
          ("jadecourt", "流光庭院", "JADE COURT", "#25364c", "#80c7b6"))
board = Image.new("RGB", (1840, 1015), "#f1ece1")
draw = ImageDraw.Draw(board)
font_path = Path("/usr/share/fonts/opentype/noto")
def text(x, y, label, size, color, bold=True):
    font = ImageFont.truetype(str(font_path / ("NotoSansCJK-Bold.ttc" if bold else "NotoSansCJK-Regular.ttc")), size, index=2)
    draw.text((x, y), label, font=font, fill=color)
text(38, 26, "AGENTX SPACE   /   GAME OFFICE STUDIES", 18, "#4e514d")
text(38, 53, "三种世界 · 同一间办公室", 43, "#202226")
for index, (key, label, tag, ink, accent) in enumerate(worlds):
    x = 38 + index * 591
    draw.rectangle((x, 133, x + 568, 137), fill=ink)
    draw.rectangle((x, 133, x + 66, 137), fill=accent)
    text(x, 150, label, 33, ink)
    text(x + 409, 163, tag, 15, ink)
    for state, y, title in (("office", 222, "场景与人物"), ("reader", 614, "会话与草稿")):
        path = source / f"{key}-{state}.png"
        with Image.open(path) as frame:
            frame.convert("RGB").save(target / f"{key}-{state}.webp", quality=91)
            shot = frame.convert("RGB")
            shot.thumbnail((568, 360), Image.Resampling.LANCZOS)
            board.paste(shot, (x, y))
            draw.rectangle((x, y, x + shot.width - 1, y + shot.height - 1), outline=ink)
        text(x, y - 29, title, 17, "#504949", False)
board.save(target / "game-office-comparison.webp", quality=93)
frames = [Image.open(p).convert("RGB") for p in sorted(source.glob("frame-*.png"))]
if len(frames) == 26:
    frames[0].save(target / "inkpost-motion.webp", format="WEBP", save_all=True, append_images=frames[1:], duration=105, loop=0, quality=88)

"""Compress full browser captures for publication without changing their composition."""
import argparse
from pathlib import Path
from PIL import Image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("captures", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    for path in sorted(args.captures.glob("*.png")):
        with Image.open(path) as capture:
            capture.convert("RGB").save(args.output / path.with_suffix(".webp").name, quality=91)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3

from pathlib import Path

import yaml


def main() -> None:
    snapcraft_path = Path(__file__).resolve().parent.parent / "snapcraft.yaml"
    with snapcraft_path.open(encoding="utf-8") as handle:
        yaml.safe_load(handle)
    print(f"{snapcraft_path.name} parses")


if __name__ == "__main__":
    main()

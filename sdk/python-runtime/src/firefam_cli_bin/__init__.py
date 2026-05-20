from __future__ import annotations

import os
from pathlib import Path

PACKAGE_NAME = "firefamai-firefam-cli-bin"


def bundled_firefam_path() -> Path:
    exe = "firefam.exe" if os.name == "nt" else "firefam"
    path = Path(__file__).resolve().parent / "bin" / exe
    if not path.is_file():
        raise FileNotFoundError(
            f"{PACKAGE_NAME} is installed but missing its packaged firefam binary at {path}"
        )
    return path


__all__ = ["PACKAGE_NAME", "bundled_firefam_path"]

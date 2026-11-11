"""Find MTGA's Player.log and card database on macOS and Windows.

Covers the Wizards installer, Steam, and Epic. Explicit paths always win:
pass --log / --card-db to the tools instead of relying on discovery.
"""

import os
import sys
from pathlib import Path


def _home():
    return Path.home()


def _log_dirs():
    """Candidate directories that contain Player.log, most likely first."""
    home = _home()
    if sys.platform == "win32":
        return [home / "AppData" / "LocalLow" / "Wizards Of The Coast" / "MTGA"]
    if sys.platform == "darwin":
        return [home / "Library" / "Logs" / "Wizards Of The Coast" / "MTGA"]
    # Steam Proton prefix on Linux
    return [
        home
        / ".steam" / "steam" / "steamapps" / "compatdata" / "2141910"
        / "pfx" / "drive_c" / "users" / "steamuser"
        / "AppData" / "LocalLow" / "Wizards Of The Coast" / "MTGA",
    ]


def _mtga_data_dirs():
    """Candidate MTGA_Data directories, most likely first."""
    home = _home()
    if sys.platform == "win32":
        program_files = Path(os.environ.get("ProgramFiles", r"C:\Program Files"))
        program_files_x86 = Path(
            os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")
        )
        return [
            program_files / "Wizards of the Coast" / "MTGA" / "MTGA_Data",
            program_files_x86 / "Steam" / "steamapps" / "common" / "MTGA" / "MTGA_Data",
            program_files / "Steam" / "steamapps" / "common" / "MTGA" / "MTGA_Data",
            program_files / "Epic Games" / "MagicTheGathering" / "MTGA_Data",
        ]
    if sys.platform == "darwin":
        return [
            home / "Library" / "Application Support" / "Steam"
            / "steamapps" / "common" / "MTGA" / "MTGA_Data",
            Path("/Applications/MTGA.app/Contents/Resources/Data"),
            home / "Applications" / "Epic Games" / "MagicTheGathering" / "MTGA_Data",
        ]
    return [
        home / ".steam" / "steam" / "steamapps" / "common" / "MTGA" / "MTGA_Data",
    ]


def find_player_log(explicit=None):
    """Return the Path to Player.log, or None if not found."""
    if explicit:
        return Path(explicit)
    for d in _log_dirs():
        p = d / "Player.log"
        if p.is_file():
            return p
    return None


def prev_player_log(player_log):
    """Return the rotated Player-prev.log Path next to a Player.log."""
    return Path(player_log).with_name("Player-prev.log")


def find_card_db(explicit=None):
    """Return the Path to the newest Raw_CardDatabase_*.mtga, or None."""
    if explicit:
        return Path(explicit)
    for data_dir in _mtga_data_dirs():
        raw = data_dir / "Downloads" / "Raw"
        if not raw.is_dir():
            continue
        candidates = sorted(raw.glob("Raw_CardDatabase_*.mtga"))
        if candidates:
            return candidates[-1]
    return None

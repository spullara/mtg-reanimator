"""MTG Arena tools.

The modules in this folder import each other with flat names (``paths``,
``parse_game``). Put this folder on sys.path so the tools also run as a
package: ``python -m arena.collect``.
"""

import sys as _sys
from pathlib import Path as _Path

_pkg_dir = str(_Path(__file__).resolve().parent)
if _pkg_dir not in _sys.path:
    _sys.path.append(_pkg_dir)

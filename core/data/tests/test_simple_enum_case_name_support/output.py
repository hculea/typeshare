"""
 Generated by typeshare 1.12.0
"""
from __future__ import annotations

from enum import Enum
from pydantic import ConfigDict


class Colors(str, Enum):
    RED = "red"
    BLUE = "blue-ish"
    GREEN = "Green"

"""
 Generated by typeshare 1.11.0
"""
from __future__ import annotations

from pydantic import BaseModel
from typing import List, Optional


class Location(BaseModel):
    pass

class Person(BaseModel):
    """
    This is a comment.
    """
    name: str
    """
    This is another comment
    """
    age: int
    info: Optional[str] = None
    emails: List[str]
    location: Location



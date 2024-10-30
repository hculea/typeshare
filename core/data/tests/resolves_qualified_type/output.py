"""
 Generated by typeshare 1.11.0
"""
from __future__ import annotations

from pydantic import BaseModel
from typing import Dict, List, Optional


class QualifiedTypes(BaseModel):
    unqualified: str
    qualified: str
    qualified_vec: List[str]
    qualified_hashmap: Dict[str, str]
    qualified_optional: Optional[str]
    qualfied_optional_hashmap_vec: Optional[Dict[str, List[str]]]



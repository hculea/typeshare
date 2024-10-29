"""
 Generated by typeshare 1.11.0
"""
from __future__ import annotations

from pydantic import BaseModel, Enum
from typing import Literal, Union


class SomeEnumTypes(str, Enum):
    A = "A"
    C = "C"

class SomeEnumA(BaseModel):
    content = Literal["A"]

class SomeEnumC(BaseModel):
    content: int

class SomeEnum(BaseModel):
    model_config = ConfigDict(use_enum_values=True)
    type: SomeEnumTypes
    content: Union[SomeEnumA, SomeEnumC]

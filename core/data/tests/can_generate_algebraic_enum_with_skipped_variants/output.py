"""
 Generated by typeshare 1.11.0
"""
from __future__ import annotations

from enum import Enum
from pydantic import BaseModel, ConfigDict
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
    content: Union[SomeEnumA, SomeEnumC, None]


    @classmethod
    def new_some_enum_a(cls) -> SomeEnum:
        return cls(
        type=SomeEnumTypes.A,
        content=None
	    )
        

    
    @classmethod
    def new_some_enum_c(cls, content : SomeEnumC):
        return cls(
            type=SomeEnumTypes.C,
            content=content
        )


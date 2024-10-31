"""
 Generated by typeshare 1.11.0
"""
from __future__ import annotations

from enum import Enum
from pydantic import BaseModel, ConfigDict, Field
from typing import Annotated, Union


class StructHasVoidType(BaseModel):
    """
    This struct has a unit field
    """
    model_config = ConfigDict(populate_by_name=True)

    this_is_a_unit: Annotated[None, Field(alias="thisIsAUnit")]


class EnumHasVoidTypeTypes(str, Enum):
    HAS_A_UNIT = "hasAUnit"

class EnumHasVoidTypeHasAUnit(BaseModel):
    content: None

class EnumHasVoidType(BaseModel):
    model_config = ConfigDict(use_enum_values=True)
    type: EnumHasVoidTypeTypes
    content: Union[EnumHasVoidTypeHasAUnit]


    @classmethod
    def new_enum_has_void_type_has_a_unit(cls, content : EnumHasVoidTypeHasAUnit):
        return cls(
            type=EnumHasVoidTypeTypes.HAS_A_UNIT,
            content=content
        )

"""
 Generated by typeshare 1.11.0
"""
from __future__ import annotations

from enum import Enum
from pydantic import BaseModel, ConfigDict, Field
from typing import Annotated, Literal, Union


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


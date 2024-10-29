"""
 Generated by typeshare 1.11.0
"""
from __future__ import annotations

from enum import Enum
from pydantic import BaseModel, ConfigDict
from typing import Union


class AutofilledByUs(BaseModel):
    """
    Generated type representing the anonymous struct variant `Us` of the `AutofilledBy` Rust enum
    """
    uuid: str
    """
    The UUID for the fill
    """


class AutofilledBySomethingElse(BaseModel):
    """
    Generated type representing the anonymous struct variant `SomethingElse` of the `AutofilledBy` Rust enum
    """
    uuid: str
    """
    The UUID for the fill
    """


class AutofilledByTypes(str, Enum):
    US = "Us"
    SOMETHING_ELSE = "SomethingElse"

class AutofilledBy(BaseModel):
    model_config = ConfigDict(use_enum_values=True)
    type: AutofilledByTypes
    content: Union[AutofilledByUs, AutofilledBySomethingElse]


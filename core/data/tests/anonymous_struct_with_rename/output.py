"""
 Generated by typeshare 1.11.0
"""
from __future__ import annotations

from enum import Enum
from pydantic import BaseModel, ConfigDict, Field
from typing import Annotated, List, Union


class AnonymousStructWithRenameList(BaseModel):
    """
    Generated type representing the anonymous struct variant `List` of the `AnonymousStructWithRename` Rust enum
    """
    list: List[str]


class AnonymousStructWithRenameLongFieldNames(BaseModel):
    """
    Generated type representing the anonymous struct variant `LongFieldNames` of the `AnonymousStructWithRename` Rust enum
    """
    model_config = ConfigDict(populate_by_name=True)

    some_long_field_name: str
    and_: Annotated[bool, Field(alias="and")]
    but_one_more: List[str]


class AnonymousStructWithRenameKebabCase(BaseModel):
    """
    Generated type representing the anonymous struct variant `KebabCase` of the `AnonymousStructWithRename` Rust enum
    """
    model_config = ConfigDict(populate_by_name=True)

    another_list: Annotated[List[str], Field(alias="another-list")]
    camel_case_string_field: Annotated[str, Field(alias="camelCaseStringField")]
    something_else: Annotated[bool, Field(alias="something-else")]


class AnonymousStructWithRenameTypes(str, Enum):
    LIST = "list"
    LONG_FIELD_NAMES = "longFieldNames"
    KEBAB_CASE = "kebabCase"

class AnonymousStructWithRename(BaseModel):
    model_config = ConfigDict(use_enum_values=True)
    type: AnonymousStructWithRenameTypes
    content: Union[AnonymousStructWithRenameList, AnonymousStructWithRenameLongFieldNames, AnonymousStructWithRenameKebabCase]


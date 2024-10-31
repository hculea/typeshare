"""
 Generated by typeshare 1.11.0
"""
from __future__ import annotations

from pydantic import BaseModel, ConfigDict, Field
from typing import Annotated, List, Optional


class OtherType(BaseModel):
    pass

class PersonTwo(BaseModel):
    """
    This is a comment.
    """
    model_config = ConfigDict(populate_by_name=True)

    name: str
    age: int
    extra_special_field_1: Annotated[int, Field(alias="extraSpecialFieldOne")]
    extra_special_field_2: Optional[Annotated[List[str], Field(alias="extraSpecialFieldTwo")]] = None
    non_standard_data_type: Annotated[OtherType, Field(alias="nonStandardDataType")]
    non_standard_data_type_in_array: Optional[Annotated[List[OtherType], Field(alias="nonStandardDataTypeInArray")]] = None



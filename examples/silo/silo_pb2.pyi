from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Optional as _Optional

DESCRIPTOR: _descriptor.FileDescriptor

class GetPackageRequest(_message.Message):
    __slots__ = ("id", "func", "args", "kwargs")
    ID_FIELD_NUMBER: _ClassVar[int]
    FUNC_FIELD_NUMBER: _ClassVar[int]
    ARGS_FIELD_NUMBER: _ClassVar[int]
    KWARGS_FIELD_NUMBER: _ClassVar[int]
    id: int
    func: str
    args: str
    kwargs: str
    def __init__(self, id: _Optional[int] = ..., func: _Optional[str] = ..., args: _Optional[str] = ..., kwargs: _Optional[str] = ...) -> None: ...

class GetPackageResponse(_message.Message):
    __slots__ = ("output", "errors")
    OUTPUT_FIELD_NUMBER: _ClassVar[int]
    ERRORS_FIELD_NUMBER: _ClassVar[int]
    output: str
    errors: str
    def __init__(self, output: _Optional[str] = ..., errors: _Optional[str] = ...) -> None: ...

import gzip
import json
from enum import Enum
from typing import Any, Dict, List, Optional, Union, Protocol, TypeVar
from dataclasses import dataclass, field


GZIP_MAGIC = bytes([0x1F, 0x8B])


class Mode(str, Enum):
    STATE = "state"
    APPEND = "append"
    LIST = "list"


class Operation(str, Enum):
    CREATE = "create"
    UPSERT = "upsert"
    PATCH = "patch"
    DELETE = "delete"


class ConnectionState(str, Enum):
    DISCONNECTED = "disconnected"
    CONNECTING = "connecting"
    CONNECTED = "connected"
    ERROR = "error"
    RECONNECTING = "reconnecting"


T = TypeVar("T")


class Entity(Protocol[T]):
    NAME: str

    @staticmethod
    def state_view() -> str:
        ...

    @staticmethod
    def list_view() -> str:
        ...


@dataclass(frozen=True)
class ViewDef:
    mode: str
    view: str


@dataclass(frozen=True)
class ViewGroup:
    state: Optional[ViewDef] = None
    list: Optional[ViewDef] = None


@dataclass(frozen=True)
class StackDefinition:
    name: str
    views: Dict[str, ViewGroup]


def state_view(view: str) -> ViewDef:
    return ViewDef(mode="state", view=view)


def list_view(view: str) -> ViewDef:
    return ViewDef(mode="list", view=view)


def is_gzip(data: bytes) -> bool:
    return len(data) >= 2 and data[:2] == GZIP_MAGIC


def parse_message(data: Union[bytes, str]) -> Dict[str, Any]:
    if isinstance(data, bytes):
        if is_gzip(data):
            decompressed = gzip.decompress(data)
            return json.loads(decompressed.decode("utf-8"))
        data = data.decode("utf-8")

    return json.loads(data)


@dataclass
class Frame:
    mode: str
    entity: str
    op: str
    key: str
    data: Dict[str, Any]
    append: List[str] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Frame":
        return cls(
            mode=data["mode"],
            entity=data.get("export") or data.get("entity", ""),
            op=data["op"],
            key=data.get("key", ""),
            data=data.get("data", {}),
            append=data.get("append", []),
        )

    @classmethod
    def from_message(cls, data: Union[bytes, str]) -> "Frame":
        parsed = parse_message(data)
        return cls(
            mode=parsed["mode"],
            entity=parsed.get("export") or parsed.get("entity", ""),
            op=parsed["op"],
            key=parsed.get("key", ""),
            data=parsed.get("data", {}),
            append=parsed.get("append", []),
        )


@dataclass
class Subscription:
    view: str
    key: Optional[str] = None
    partition: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        result: Dict[str, Any] = {"type": "subscribe", "view": self.view}
        if self.key is not None:
            result["key"] = self.key
        if self.partition is not None:
            result["partition"] = self.partition
        return result

    def sub_key(self) -> str:
        return f"{self.view}:{self.key or '*'}"


@dataclass
class Unsubscription:
    view: str
    key: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        result: Dict[str, Any] = {"type": "unsubscribe", "view": self.view}
        if self.key is not None:
            result["key"] = self.key
        return result

    def sub_key(self) -> str:
        return f"{self.view}:{self.key or '*'}"

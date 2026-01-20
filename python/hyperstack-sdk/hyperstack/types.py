import base64
import gzip
import json
from enum import Enum
from typing import Any, Dict, List, Optional
from dataclasses import dataclass, field


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


def decompress_frame(data: Dict[str, Any]) -> Dict[str, Any]:
    if data.get("compressed") == "gzip" and "data" in data:
        compressed_bytes = base64.b64decode(data["data"])
        decompressed = gzip.decompress(compressed_bytes)
        return json.loads(decompressed.decode("utf-8"))
    return data


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
        data = decompress_frame(data)
        return cls(
            mode=data["mode"],
            entity=data.get("export") or data.get("entity", ""),
            op=data["op"],
            key=data.get("key", ""),
            data=data.get("data", {}),
            append=data.get("append", []),
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

from enum import Enum
from typing import Any, Dict, Optional
from dataclasses import dataclass


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


@dataclass
class Frame:
    mode: str
    entity: str
    op: str
    key: str
    data: Dict[str, Any]

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Frame":
        return cls(
            mode=data["mode"],
            entity=data.get("export") or data.get("entity", ""),
            op=data["op"],
            key=data["key"],
            data=data.get("data", {}),
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

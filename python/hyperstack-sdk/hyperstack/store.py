import asyncio
import logging
from collections import OrderedDict
from typing import (
    Any,
    Dict,
    Optional,
    TypeVar,
    AsyncIterator,
    Generic,
    Callable,
    List,
    Union,
)
from dataclasses import dataclass
from enum import Enum

logger = logging.getLogger(__name__)

T = TypeVar("T")

DEFAULT_MAX_ENTRIES_PER_VIEW = 10_000


class Mode(Enum):
    STATE = "state"
    LIST = "list"
    APPEND = "append"


@dataclass
class Update(Generic[T]):
    key: str
    data: T


class Store(Generic[T]):
    def __init__(
        self,
        mode: Mode = Mode.LIST,
        parser: Optional[Callable[[Dict[str, Any]], T]] = None,
        view: Optional[str] = None,
        max_entries: Optional[int] = DEFAULT_MAX_ENTRIES_PER_VIEW,
    ):
        self.mode = mode
        self.parser = parser
        self.view = view
        self.max_entries = max_entries
        self._lock = asyncio.Lock()
        self._callbacks: List[Callable[[Update[T]], None]] = []
        self._update_queue: asyncio.Queue[Update[T]] = asyncio.Queue()

        if mode in (Mode.LIST, Mode.STATE):
            self._data: Union[OrderedDict[str, T], List[T]] = OrderedDict()
        else:
            self._data: Union[OrderedDict[str, T], List[T]] = []

    def _enforce_max_entries(self) -> None:
        if self.max_entries is None:
            return
        if isinstance(self._data, OrderedDict):
            while len(self._data) > self.max_entries:
                self._data.popitem(last=False)

    def get(self, key: Optional[str] = None) -> Optional[T]:
        if isinstance(self._data, OrderedDict):
            if key is None:
                return next(iter(self._data.values()), None)
            if key in self._data:
                self._data.move_to_end(key)
            return self._data.get(key)
        else:
            return self._data[0] if self._data else None

    async def get_async(self, key: Optional[str] = None) -> Optional[T]:
        async with self._lock:
            return self.get(key)

    def subscribe(self, callback: Callable[[Update[T]], None]) -> None:
        if callback not in self._callbacks:
            self._callbacks.append(callback)

    def unsubscribe(self, callback: Callable[[Update[T]], None]) -> None:
        if callback in self._callbacks:
            self._callbacks.remove(callback)

    def _parse_data(self, data: Any) -> T:
        """Parse raw data using the custom parser if provided."""
        if self.parser:
            return self.parser(data)
        return data

    async def __aiter__(self) -> AsyncIterator[Update[T]]:
        while True:
            update = await self._update_queue.get()
            yield update

    async def apply_patch(
        self, key: str, patch: Dict[str, Any], append_paths: Optional[List[str]] = None
    ) -> None:
        if append_paths is None:
            append_paths = []
        async with self._lock:
            if isinstance(self._data, OrderedDict):
                current = self._data.get(key, {})
                if isinstance(current, dict):
                    merged = self._deep_merge_with_append(current, patch, append_paths)
                else:
                    merged = patch
                parsed_data = self._parse_data(merged)
                self._data[key] = parsed_data
                self._data.move_to_end(key)
                self._enforce_max_entries()
                await self._notify_update(key, parsed_data)
            else:
                parsed_data = self._parse_data(patch)
                self._data.append(parsed_data)
                await self._notify_update(key, parsed_data)

    def _deep_merge_with_append(
        self,
        target: Dict[str, Any],
        source: Dict[str, Any],
        append_paths: List[str],
        current_path: str = "",
    ) -> Dict[str, Any]:
        result = {**target}
        for key, source_value in source.items():
            field_path = f"{current_path}.{key}" if current_path else key
            target_value = result.get(key)

            if isinstance(source_value, list) and isinstance(target_value, list):
                if field_path in append_paths:
                    result[key] = target_value + source_value
                else:
                    result[key] = source_value
            elif isinstance(source_value, dict) and isinstance(target_value, dict):
                result[key] = self._deep_merge_with_append(
                    target_value, source_value, append_paths, field_path
                )
            else:
                result[key] = source_value
        return result

    async def apply_upsert(self, key: str, value: T) -> None:
        async with self._lock:
            parsed_data = self._parse_data(value)
            if isinstance(self._data, OrderedDict):
                self._data[key] = parsed_data
                self._data.move_to_end(key)
                self._enforce_max_entries()
            else:
                self._data.append(parsed_data)

            await self._notify_update(key, parsed_data)

    async def apply_delete(self, key: str) -> None:
        async with self._lock:
            if isinstance(self._data, OrderedDict) and key in self._data:
                del self._data[key]
                await self._notify_update(key, None)  # type: ignore[arg-type]

    async def handle_frame(self, frame) -> None:
        if frame.op == "upsert":
            await self.apply_upsert(frame.key, frame.data)
        elif frame.op == "patch":
            append_paths = getattr(frame, "append", []) or []
            await self.apply_patch(frame.key, frame.data, append_paths)
        elif frame.op == "delete":
            await self.apply_delete(frame.key)
        else:
            await self.apply_upsert(frame.key, frame.data)

    async def _notify_update(self, key: str, data: T) -> None:
        update = Update(key=key, data=data)

        await self._update_queue.put(update)

        # Call all registered callbacks
        for callback in self._callbacks:
            try:
                callback(update)
            except Exception as e:
                logger.error(f"Callback error: {e}")

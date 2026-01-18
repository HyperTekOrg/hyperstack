import asyncio
import logging
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
    ):
        self.mode = mode
        self.parser = parser
        self.view = view
        self._lock = asyncio.Lock()
        self._callbacks: List[Callable[[Update[T]], None]] = []
        self._update_queue: asyncio.Queue[Update[T]] = asyncio.Queue()

        if mode in (Mode.LIST, Mode.STATE):
            self._data: Union[Dict[str, T], List[T]] = {}
        else:
            self._data: Union[Dict[str, T], List[T]] = []

    def get(self, key: Optional[str] = None) -> Optional[T]:
        """
        Get current value (synchronous).

        Args:
            key: Key to retrieve. For dict-based modes, returns the value at key.
                 For list-based modes, returns the first item if no key specified.

        Returns:
            The stored value if found, None otherwise
        """
        if isinstance(self._data, dict):
            if key is None:
                return next(iter(self._data.values()), None)
            return self._data.get(key)
        else:
            return self._data[0] if self._data else None

    async def get_async(self, key: Optional[str] = None) -> Optional[T]:
        """
        Get current value (asynchronous, waits if not available).

        Args:
            key: Key to retrieve

        Returns:
            The stored value if found, None otherwise
        """
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
            if isinstance(self._data, dict):
                current = self._data.get(key, {})
                if isinstance(current, dict):
                    merged = self._deep_merge_with_append(current, patch, append_paths)
                else:
                    merged = patch
                parsed_data = self._parse_data(merged)
                self._data[key] = parsed_data
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
            if isinstance(self._data, dict):
                # List/State mode: direct insert/update
                self._data[key] = parsed_data
            else:
                # Append mode: just append
                self._data.append(parsed_data)

            await self._notify_update(key, parsed_data)

    async def apply_delete(self, key: str) -> None:
        async with self._lock:
            if isinstance(self._data, dict) and key in self._data:
                del self._data[key]
                await self._notify_update(key, None)

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

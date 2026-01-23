import asyncio
import json
import logging
from typing import Dict, List, Optional, Callable

from hyperstack.connection import ConnectionManager
from hyperstack.store import Store, Mode, SharedStore
from hyperstack.types import (
    Subscription,
    Unsubscription,
    Frame,
    Entity,
    StackDefinition,
    ConnectionState,
)
from hyperstack.views import create_typed_views, TypedViews

logger = logging.getLogger(__name__)


def parse_mode(view: str) -> Mode:
    if view.endswith("/state"):
        return Mode.STATE
    elif view.endswith("/list"):
        return Mode.LIST
    elif view.endswith("/append"):
        return Mode.APPEND
    else:
        return Mode.LIST  # Default to list mode


class HyperStackClient:
    def __init__(
        self,
        url: str,
        reconnect_intervals: List[int] = [1, 2, 4, 8, 16],
        ping_interval: int = 15,
        on_connect: Optional[Callable] = None,
        on_disconnect: Optional[Callable] = None,
        on_error: Optional[Callable] = None,
    ):
        self.url = url
        self._store = SharedStore()
        self._pending_subs: List[Subscription] = []
        self._active_subs: Dict[str, Subscription] = {}
        self._user_on_connect = on_connect
        self.ws_manager = ConnectionManager(
            url=url,
            reconnect_intervals=reconnect_intervals,
            ping_interval=ping_interval,
            on_connect=self._on_connect,
            on_disconnect=on_disconnect,
            on_error=on_error,
        )
        self.ws_manager.set_message_handler(self._on_message)

    async def connect(self) -> None:
        """Connect to HyperStack server."""
        await self.ws_manager.connect()

    async def disconnect(self) -> None:
        """Disconnect from server."""
        await self.ws_manager.disconnect()

    async def __aenter__(self):
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.disconnect()

    def subscribe(
        self, view: str, key: Optional[str] = None, parser: Optional[Callable] = None
    ) -> Store:
        """
        # Subscribes to updates for the specified view (and optional key) on the HyperStack server.
        #
        # Args:
        #     view (str): The view to subscribe to, in the format 'Entity/mode'.
        #     key (Optional[str]): An optional key to filter the subscription to a specific entity or item.
        #     parser (Optional[Callable]): An optional parser function to transform raw data into custom types.
        #
        # Returns:
        #     Store: A Store instance that provides access to real-time updates for the subscribed view.
        """
        if "/" not in view:
            raise ValueError(f"Invalid view '{view}'. Expected: Entity/mode")

        mode = parse_mode(view)
        store = self._store.get_store(view, mode=mode, parser=parser)

        sub = Subscription(view=view, key=key)
        sub_key = sub.sub_key()
        if sub_key not in self._active_subs:
            self._active_subs[sub_key] = sub
            if self.ws_manager.is_running:
                asyncio.create_task(self._send_sub(sub))
            else:
                self._pending_subs.append(sub)

        return store

    async def get(
        self,
        entity: Entity,
        key: str,
        parser: Optional[Callable] = None,
        timeout: Optional[float] = None,
    ) -> Optional[Dict]:
        view = entity.state_view()
        self.subscribe(view, key=key, parser=parser)
        await self._store.wait_for_view_ready(view, timeout=timeout)
        return await self._store.get(entity.state_view(), key)

    async def list(
        self,
        entity: Entity,
        parser: Optional[Callable] = None,
        timeout: Optional[float] = None,
    ) -> List:
        view = entity.list_view()
        self.subscribe(view, parser=parser)
        await self._store.wait_for_view_ready(view, timeout=timeout)
        return await self._store.list(entity.list_view())

    def watch(self, entity: Entity, parser: Optional[Callable] = None) -> Store:
        return self.subscribe(entity.list_view(), parser=parser)

    def watch_key(
        self, entity: Entity, key: str, parser: Optional[Callable] = None
    ) -> Store:
        return self.subscribe(entity.list_view(), key=key, parser=parser)

    def views(self, stack: StackDefinition) -> TypedViews:
        return create_typed_views(stack, self)

    async def _on_connect(self) -> None:
        """Send queued subscriptions on connect."""
        for sub in self._active_subs.values():
            await self._send_sub(sub)
        self._pending_subs.clear()

        if self._user_on_connect:
            await self._user_on_connect()

    async def _send_sub(self, sub: Subscription) -> None:
        """Send subscription to server."""
        if not self.ws_manager.ws or not self.ws_manager.is_running:
            return

        try:
            await self.ws_manager.ws.send(json.dumps(sub.to_dict()))
            logger.info(f"Subscribed: {sub.view}")
        except Exception as e:
            logger.error(f"Subscribe failed: {e}")

    async def unsubscribe(self, view: str, key: Optional[str] = None) -> None:
        """Unsubscribe from a view."""
        sub = Subscription(view=view, key=key)
        sub_key = sub.sub_key()
        self._active_subs.pop(sub_key, None)
        self._pending_subs = [s for s in self._pending_subs if s.sub_key() != sub_key]

        if not self.ws_manager.ws or not self.ws_manager.is_running:
            return

        try:
            unsub = Unsubscription(view=view, key=key)
            await self.ws_manager.ws.send(json.dumps(unsub.to_dict()))
            logger.info(f"Unsubscribed: {view}")
        except Exception as e:
            logger.error(f"Unsubscribe failed: {e}")

    async def _on_message(self, message) -> None:
        """
        Processes incoming WebSocket messages received from the HyperStack server.

        Decodes the incoming message, parses it into a Frame object,
        and dispatches it to the appropriate Store instance(s) based on the view and key.
        It ensures that each relevant Store receives real-time updates and handles them accordingly.

        Args:
            message: The incoming WebSocket message, either as a string or bytes.

        Returns:
            None
        """
        try:
            frame = Frame.from_message(message)
            logger.debug(
                f"Frame: entity={frame.entity}, op={frame.op}, key={frame.key}"
            )
            await self._store.apply_frame(frame)

        except Exception as e:
            logger.error(f"Message error: {e}", exc_info=True)

    def store(self) -> SharedStore:
        return self._store

    def connection_state(self) -> ConnectionState:
        return self.ws_manager.state()


import asyncio
import json
import logging
from typing import Dict, List, Optional, Callable

from hyperstack.websocket import WebSocketManager
from hyperstack.store import Store, Mode
from hyperstack.types import Subscription, Frame

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
        self._stores: Dict[str, Store] = {}
        self._pending_subs: List[Subscription] = []
        self._user_on_connect = on_connect

        self.ws_manager = WebSocketManager(
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
        store = Store(mode=mode, parser=parser, view=view)

        store_key = f"{view}:{key or '*'}"
        self._stores[store_key] = store

        sub = Subscription(view=view, key=key)
        if self.ws_manager.is_running:
            asyncio.create_task(self._send_sub(sub))
        else:
            self._pending_subs.append(sub)

        return store

    async def _on_connect(self) -> None:
        """Send queued subscriptions on connect."""
        while self._pending_subs:
            await self._send_subscription(self._pending_subs.pop(0))

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
            text = message.decode("utf-8") if isinstance(message, bytes) else message

            frame = Frame.from_dict(json.loads(text))
            logger.debug(
                f"Frame: entity={frame.entity}, op={frame.op}, key={frame.key}"
            )

            view = frame.entity
            store_keys = [f"{view}:{frame.key}", f"{view}:*"]

            for store_key in store_keys:
                store = self._stores.get(store_key)
                if store:
                    logger.debug(f"Routing to: {store_key}")
                    await store.handle_frame(frame)

        except Exception as e:
            logger.error(f"Message error: {e}", exc_info=True)

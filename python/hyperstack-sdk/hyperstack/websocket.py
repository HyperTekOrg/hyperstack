import asyncio
import json
import logging
from typing import Optional, Callable, List, Any
from websockets import connect as ws_connect
from websockets.client import WebSocketClientProtocol
from websockets.exceptions import WebSocketException

from hyperstack.errors import ConnectionError

logger = logging.getLogger(__name__)


class WebSocketManager:
    def __init__(
        self,
        url: str,
        reconnect_intervals: List[int] = [1, 2, 4, 8, 16],
        ping_interval: int = 15,
        on_connect: Optional[Callable] = None,
        on_disconnect: Optional[Callable] = None,
        on_error: Optional[Callable] = None,
    ):
        """
        Initialize WebSocket manager.

        Args:
            url: WebSocket server URL
            reconnect_intervals: List of wait intervals (in seconds) between reconnection attempts
            on_connect: Optional callback invoked when connection is established
            on_disconnect: Optional callback invoked when connection is closed
            on_error: Optional callback invoked when an error occurs
        """
        self.url = url
        self.reconnect_intervals = reconnect_intervals
        self.ping_interval = ping_interval
        self.on_connect = on_connect
        self.on_disconnect = on_disconnect
        self.on_error = on_error

        self.ws: Optional[WebSocketClientProtocol] = None
        self.is_running = False
        self.reconnect_attempts = 0
        self.receive_task: Optional[asyncio.Task] = None
        self.ping_task: Optional[asyncio.Task] = None
        self.message_handler: Optional[Callable] = None

    def set_message_handler(self, handler: Callable) -> None:
        """
        Set the callback function for handling incoming WebSocket messages.

        Args:
            handler: Async callable that processes incoming messages
        """
        self.message_handler = handler

    async def _ping_loop(self) -> None:
        while self.is_running and self.ws:
            await asyncio.sleep(self.ping_interval)
            if self.is_running and self.ws:
                try:
                    await self.ws.send('{"type":"ping"}')
                    logger.debug("Sent keep-alive ping")
                except Exception as e:
                    logger.warning(f"Ping failed: {e}")
                    break

    def _start_ping(self) -> None:
        self._stop_ping()
        self.ping_task = asyncio.create_task(self._ping_loop())

    def _stop_ping(self) -> None:
        if self.ping_task:
            self.ping_task.cancel()
            self.ping_task = None

    async def connect(self) -> None:
        """
        Establish WebSocket connection with automatic retry logic.

        Attempts to connect using the configured reconnect intervals.
        Starts the message receiving loop once connected.

        Raises:
            ConnectionError: If all connection attempts fail
        """
        if self.is_running and self.ws:
            logger.info("Already connected")
            return

        attempt = 0
        while attempt < len(self.reconnect_intervals):
            try:
                logger.info(f"Connecting to {self.url}...")
                self.ws = await ws_connect(self.url)
                self.is_running = True
                self.reconnect_attempts = 0
                logger.info("Connected")

                self.receive_task = asyncio.create_task(self.receive_messages())
                self._start_ping()

                if self.on_connect:
                    await self.on_connect()

                return

            except Exception as e:
                attempt += 1
                if attempt >= len(self.reconnect_intervals):
                    raise ConnectionError(
                        f"Connection failed after {attempt} attempts: {e}"
                    )

                wait = self.reconnect_intervals[attempt - 1]
                logger.warning(f"Retrying in {wait}s (attempt {attempt})")
                await asyncio.sleep(wait)

        raise ConnectionError("Failed to connect")

    async def disconnect(self) -> None:
        """Close WebSocket connection and cleanup resources."""
        self.is_running = False
        self._stop_ping()

        if self.receive_task:
            self.receive_task.cancel()
            try:
                await self.receive_task
            except asyncio.CancelledError:
                pass

        if self.ws:
            await self.ws.close()
            self.ws = None

        if self.on_disconnect:
            await self.on_disconnect()

        logger.info("Disconnected")

    async def receive_messages(self) -> None:
        """
        Continuously receive and process WebSocket messages.

        Handles incoming messages via the configured message handler.
        Automatically triggers reconnection on WebSocket errors if still running.
        """
        if not self.ws:
            return

        try:
            async for message in self.ws:
                if self.message_handler:
                    try:
                        await self.message_handler(message)
                    except Exception as e:
                        logger.error(f"Frame error: {e}")
                        if self.on_error:
                            await self.on_error(e)

        except WebSocketException as e:
            logger.error(f"WebSocket error: {e}")
            if self.on_error:
                await self.on_error(e)

            self._stop_ping()
            if self.is_running:
                await self.handle_reconnect()

        except Exception as e:
            logger.error(f"Receive error: {e}")
            if self.on_error:
                await self.on_error(e)

    async def handle_reconnect(self) -> None:
        self.reconnect_attempts += 1

        if self.reconnect_attempts > len(self.reconnect_intervals):
            logger.error("Max reconnect attempts reached")
            return

        wait = self.reconnect_intervals[self.reconnect_attempts - 1]
        logger.info(f"Reconnecting in {wait}s (attempt {self.reconnect_attempts})")
        await asyncio.sleep(wait)
        await self.connect()

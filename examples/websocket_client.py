#!/usr/bin/env python3
"""
WebSocket Client Example for RustyDB

This example demonstrates how to connect to RustyDB using WebSocket
for real-time query execution and streaming results.

Requirements:
    pip install websockets asyncio

Usage:
    python websocket_client.py
"""

import asyncio
import json
import uuid
from datetime import datetime, timezone
from typing import Optional, Dict, Any
import websockets
from websockets.client import WebSocketClientProtocol


class RustyDBWebSocketClient:
    """WebSocket client for RustyDB with automatic reconnection and error handling."""

    def __init__(
        self,
        url: str = "ws://localhost:8080/api/v1/stream",
        auth_token: Optional[str] = None,
        auto_reconnect: bool = True,
        max_reconnect_attempts: int = 5,
    ):
        """
        Initialize the WebSocket client.

        Args:
            url: WebSocket server URL
            auth_token: JWT authentication token (optional)
            auto_reconnect: Enable automatic reconnection
            max_reconnect_attempts: Maximum number of reconnection attempts
        """
        self.url = url
        self.auth_token = auth_token
        self.auto_reconnect = auto_reconnect
        self.max_reconnect_attempts = max_reconnect_attempts
        self.websocket: Optional[WebSocketClientProtocol] = None
        self.reconnect_count = 0
        self.authenticated = False

    async def connect(self) -> None:
        """Establish WebSocket connection to RustyDB."""
        print(f"Connecting to: {self.url}")

        try:
            self.websocket = await websockets.connect(self.url)
            print("✓ Connected successfully")

            # Authenticate if token is provided
            if self.auth_token:
                await self.authenticate()

            self.reconnect_count = 0

        except Exception as e:
            print(f"✗ Connection failed: {e}")
            raise

    async def authenticate(self) -> bool:
        """Authenticate with the server using JWT token."""
        if not self.auth_token:
            print("⚠ No authentication token provided")
            return False

        auth_message = {
            "type": "auth",
            "payload": {"token": self.auth_token},
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "id": str(uuid.uuid4()),
        }

        await self.send_message(auth_message)
        print("✓ Sent authentication message")

        # Wait for authentication response
        response = await self.receive_message()
        if response and response["type"] == "auth_success":
            print("✓ Authentication successful")
            self.authenticated = True
            return True
        elif response and response["type"] == "auth_error":
            print(f"✗ Authentication failed: {response.get('payload', {})}")
            self.authenticated = False
            return False

        return False

    async def send_message(self, message: Dict[str, Any]) -> None:
        """Send a message to the server."""
        if not self.websocket:
            raise RuntimeError("Not connected to server")

        message_json = json.dumps(message)
        await self.websocket.send(message_json)

    async def receive_message(self, timeout: Optional[float] = 5.0) -> Optional[Dict[str, Any]]:
        """Receive a message from the server with optional timeout."""
        if not self.websocket:
            raise RuntimeError("Not connected to server")

        try:
            if timeout:
                message_text = await asyncio.wait_for(
                    self.websocket.recv(), timeout=timeout
                )
            else:
                message_text = await self.websocket.recv()

            return json.loads(message_text)

        except asyncio.TimeoutError:
            print("⚠ Timeout waiting for response")
            return None
        except json.JSONDecodeError as e:
            print(f"✗ Failed to parse JSON: {e}")
            return None

    async def execute_query(
        self, sql: str, params: Optional[list] = None, streaming: bool = True
    ) -> Optional[Dict[str, Any]]:
        """
        Execute a SQL query and receive results.

        Args:
            sql: SQL query to execute
            params: Query parameters (optional)
            streaming: Enable streaming results

        Returns:
            Query response dictionary
        """
        query_message = {
            "type": "query",
            "payload": {
                "sql": sql,
                "params": params or [],
                "streaming": streaming,
            },
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "id": str(uuid.uuid4()),
        }

        await self.send_message(query_message)
        return await self.receive_message()

    async def ping(self) -> bool:
        """Send a ping message and wait for pong response."""
        ping_message = {
            "type": "ping",
            "payload": {},
            "timestamp": datetime.now(timezone.utc).isoformat(),
        }

        await self.send_message(ping_message)

        response = await self.receive_message(timeout=5.0)
        return response is not None and response["type"] == "pong"

    async def close(self) -> None:
        """Close the WebSocket connection."""
        if self.websocket:
            await self.websocket.close()
            print("✓ Connection closed")
            self.websocket = None
            self.authenticated = False

    async def __aenter__(self):
        """Async context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.close()


async def example_1_basic_connection():
    """Example 1: Basic connection and authentication."""
    print("\n" + "=" * 50)
    print("Example 1: Basic Connection")
    print("=" * 50 + "\n")

    # Create client without authentication
    async with RustyDBWebSocketClient() as client:
        print("✓ Connection established and closed successfully")


async def example_2_authenticated_connection():
    """Example 2: Authenticated connection."""
    print("\n" + "=" * 50)
    print("Example 2: Authenticated Connection")
    print("=" * 50 + "\n")

    # In production, you would get this token from the login endpoint
    auth_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

    async with RustyDBWebSocketClient(auth_token=auth_token) as client:
        if client.authenticated:
            print("✓ Authenticated connection ready for queries")
        else:
            print("⚠ Connected but not authenticated")


async def example_3_query_execution():
    """Example 3: Execute queries and receive results."""
    print("\n" + "=" * 50)
    print("Example 3: Query Execution")
    print("=" * 50 + "\n")

    async with RustyDBWebSocketClient() as client:
        # Execute a simple query
        sql = "SELECT name, age FROM users WHERE age > 18 ORDER BY name LIMIT 5"
        print(f"Executing query: {sql}")

        response = await client.execute_query(sql)

        if response and response["type"] == "data":
            payload = response["payload"]
            print(f"✓ Query executed successfully")
            print(f"  Status: {payload.get('status')}")
            print(f"  Columns: {payload.get('columns', [])}")
            print(f"  Rows returned: {len(payload.get('rows', []))}")

            # Print results
            rows = payload.get("rows", [])
            if rows:
                print("\n  Results:")
                for i, row in enumerate(rows, 1):
                    print(f"    Row {i}: {row}")

        elif response and response["type"] == "error":
            print(f"✗ Query error: {response['payload'].get('message')}")


async def example_4_heartbeat():
    """Example 4: Heartbeat (ping/pong)."""
    print("\n" + "=" * 50)
    print("Example 4: Heartbeat")
    print("=" * 50 + "\n")

    async with RustyDBWebSocketClient() as client:
        print("Sending ping...")
        if await client.ping():
            print("✓ Received pong response")
        else:
            print("✗ No pong response received")


async def example_5_error_handling():
    """Example 5: Error handling for invalid queries."""
    print("\n" + "=" * 50)
    print("Example 5: Error Handling")
    print("=" * 50 + "\n")

    async with RustyDBWebSocketClient() as client:
        # Execute an invalid query
        sql = "SELECT * FROM nonexistent_table"
        print(f"Executing invalid query: {sql}")

        response = await client.execute_query(sql)

        if response and response["type"] == "error":
            print("✓ Received expected error response")
            print(f"  Error message: {response['payload'].get('message')}")
            print(f"  Error code: {response['payload'].get('code')}")
        else:
            print("⚠ Expected error but got different response")


async def example_6_multiple_queries():
    """Example 6: Execute multiple queries sequentially."""
    print("\n" + "=" * 50)
    print("Example 6: Multiple Queries")
    print("=" * 50 + "\n")

    async with RustyDBWebSocketClient() as client:
        queries = [
            "SELECT COUNT(*) FROM users",
            "SELECT AVG(age) FROM users",
            "SELECT MAX(created_at) FROM users",
        ]

        for i, sql in enumerate(queries, 1):
            print(f"\nQuery {i}: {sql}")
            response = await client.execute_query(sql)

            if response and response["type"] == "data":
                payload = response["payload"]
                rows = payload.get("rows", [])
                if rows:
                    print(f"  Result: {rows[0]}")
            else:
                print(f"  Error: {response}")


async def example_7_reconnection():
    """Example 7: Automatic reconnection on failure."""
    print("\n" + "=" * 50)
    print("Example 7: Reconnection Logic")
    print("=" * 50 + "\n")

    client = RustyDBWebSocketClient(
        auto_reconnect=True,
        max_reconnect_attempts=3
    )

    max_retries = 3
    retry_count = 0
    retry_delay = 1.0

    while retry_count < max_retries:
        try:
            await client.connect()
            print("✓ Connected successfully")

            # Simulate some work
            await asyncio.sleep(1)

            break

        except Exception as e:
            retry_count += 1
            if retry_count >= max_retries:
                print(f"✗ Max retries ({max_retries}) reached, giving up")
                break

            print(f"✗ Connection failed (attempt {retry_count}/{max_retries}): {e}")
            print(f"  Retrying in {retry_delay}s...")
            await asyncio.sleep(retry_delay)

            # Exponential backoff
            retry_delay = min(retry_delay * 2, 60.0)

    await client.close()


async def example_8_streaming_results():
    """Example 8: Handle streaming results from large queries."""
    print("\n" + "=" * 50)
    print("Example 8: Streaming Results")
    print("=" * 50 + "\n")

    async with RustyDBWebSocketClient() as client:
        # Execute a query that returns a large result set
        sql = "SELECT * FROM large_table LIMIT 1000"
        print(f"Executing query: {sql}")

        await client.send_message({
            "type": "query",
            "payload": {
                "sql": sql,
                "streaming": True,
            },
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "id": str(uuid.uuid4()),
        })

        # Receive streaming results
        total_rows = 0
        while True:
            response = await client.receive_message(timeout=10.0)

            if not response:
                print("⚠ No more messages")
                break

            if response["type"] == "data":
                payload = response["payload"]
                rows = payload.get("rows", [])
                total_rows += len(rows)
                print(f"  Received batch: {len(rows)} rows (total: {total_rows})")

                # Check if there are more results
                if not payload.get("has_more", False):
                    print("✓ All results received")
                    break

            elif response["type"] == "error":
                print(f"✗ Error: {response['payload'].get('message')}")
                break


async def main():
    """Run all examples."""
    print("=" * 50)
    print("RustyDB WebSocket Client Examples")
    print("=" * 50)

    # Run examples
    try:
        await example_1_basic_connection()
        await example_2_authenticated_connection()
        await example_3_query_execution()
        await example_4_heartbeat()
        await example_5_error_handling()
        await example_6_multiple_queries()
        await example_7_reconnection()
        await example_8_streaming_results()

        print("\n" + "=" * 50)
        print("All examples completed successfully!")
        print("=" * 50)

    except KeyboardInterrupt:
        print("\n\n✗ Interrupted by user")
    except Exception as e:
        print(f"\n\n✗ Error running examples: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    # Run the async main function
    asyncio.run(main())

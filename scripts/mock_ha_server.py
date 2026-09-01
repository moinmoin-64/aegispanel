#!/usr/bin/env python3
"""
AegisPanel OS - Mock Home Assistant & Alarmo WebSocket Test Server
Simulates HA WebSocket API for testing aegispanel-core integration.
"""

import asyncio
import json
import websockets
import sys

PORT = 8123
VALID_PIN = "1234"
ALARMO_STATE = "armed_home"

async def ha_websocket_handler(websocket, path):
    global ALARMO_STATE
    print(f"[Mock HA] Client connected from {websocket.remote_address}")

    # Step 1: Send auth_required
    await websocket.send(json.dumps({"type": "auth_required", "ha_version": "2024.8.0"}))

    # Step 2: Receive auth
    auth_msg = await websocket.recv()
    auth_data = json.loads(auth_msg)
    if auth_data.get("type") == "auth":
        print("[Mock HA] Authentication SUCCESS")
        await websocket.send(json.dumps({"type": "auth_ok", "ha_version": "2024.8.0"}))
    else:
        print("[Mock HA] Authentication FAILED")
        await websocket.send(json.dumps({"type": "auth_invalid", "message": "Invalid token"}))
        return

    # Step 3: Message Loop
    async for message in websocket:
        data = json.loads(message)
        msg_type = data.get("type")
        msg_id = data.get("id")

        if msg_type == "ping":
            await websocket.send(json.dumps({"id": msg_id, "type": "pong"}))
        elif msg_type == "get_states":
            await websocket.send(json.dumps({
                "id": msg_id,
                "type": "result",
                "success": True,
                "result": [
                    {
                        "entity_id": "alarm_control_panel.alarmo",
                        "state": ALARMO_STATE,
                        "attributes": {"arm_mode": "armed_home"}
                    }
                ]
            }))
        elif msg_type == "subscribe_events":
            await websocket.send(json.dumps({"id": msg_id, "type": "result", "success": True}))
            print("[Mock HA] Client subscribed to state_changed events")
        elif msg_type == "call_service":
            domain = data.get("domain")
            service = data.get("service")
            service_data = data.get("service_data", {})
            pin = service_data.get("code")

            print(f"[Mock HA] Service Call: {domain}.{service} with PIN code: {pin}")

            if domain == "alarm_control_panel" and service == "alarm_disarm":
                if pin == VALID_PIN:
                    ALARMO_STATE = "disarmed"
                    print("[Mock HA] PIN CORRECT! Disarming Alarmo...")
                    await websocket.send(json.dumps({"id": msg_id, "type": "result", "success": True}))

                    # Broadcast state_changed event
                    event_msg = {
                        "type": "event",
                        "event": {
                            "event_type": "state_changed",
                            "data": {
                                "entity_id": "alarm_control_panel.alarmo",
                                "new_state": {"state": "disarmed"}
                            }
                        }
                    }
                    await websocket.send(json.dumps(event_msg))
                else:
                    print(f"[Mock HA] PIN INCORRECT ({pin})!")
                    await websocket.send(json.dumps({
                        "id": msg_id,
                        "type": "result",
                        "success": False,
                        "error": {"code": "invalid_code", "message": "Falscher Alarmo PIN"}
                    }))

async def main():
    print(f"[Mock HA] Starting Home Assistant WebSocket Server on ws://localhost:{PORT}/api/websocket")
    async with websockets.serve(ha_websocket_handler, "0.0.0.0", PORT):
        await asyncio.Future()  # run forever

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("[Mock HA] Stopped.")

"""Expose one owner-admitted text packet as a local MCP stdio tool."""

import argparse
import json
from pathlib import Path
import sys

from packet import MAX_PACKET_BYTES, load_packet

PROTOCOLS = ("2025-11-25", "2025-06-18")
MAX_REQUEST_BYTES = 16 * 1024
TOOL = {
    "name": "read_original",
    "description": "Read one selected captured source by slot and packet digest.",
    "inputSchema": {
        "type": "object",
        "properties": {"slot": {"type": "string"}, "packet_sha256": {"type": "string"}},
        "required": ["slot", "packet_sha256"],
        "additionalProperties": False,
    },
    "annotations": {"readOnlyHint": True},
}


def serve(read, source, destination):
    negotiated = False
    ready = False
    while line := source.readline(MAX_REQUEST_BYTES + 1):
        if len(line) > MAX_REQUEST_BYTES:
            return
        response = {"jsonrpc": "2.0", "id": None}
        try:
            request = json.loads(line.decode("utf-8"))
        except (UnicodeError, ValueError, RecursionError):
            response["error"] = {"code": -32700, "message": "Parse error"}
        else:
            if isinstance(request, dict) and type(request.get("id")) in (str, int):
                response["id"] = request["id"]
            method = request.get("method") if isinstance(request, dict) else None
            if (
                not isinstance(method, str)
                or request.get("jsonrpc") != "2.0"
                or ("id" in request and response["id"] is None)
            ):
                response["error"] = {"code": -32600, "message": "Invalid request"}
            elif "id" not in request:
                if method == "notifications/initialized" and negotiated:
                    ready = True
                continue
            else:
                params = request.get("params", {})
                if not isinstance(params, dict):
                    response["error"] = {"code": -32602, "message": "Invalid params"}
                elif method == "initialize" and not negotiated:
                    requested = params.get("protocolVersion")
                    if not isinstance(requested, str):
                        response["error"] = {"code": -32602, "message": "Invalid protocol version"}
                    else:
                        negotiated = True
                        response["result"] = {
                            "protocolVersion": requested if requested in PROTOCOLS else PROTOCOLS[0],
                            "capabilities": {"tools": {}},
                            "serverInfo": {"name": "pale-review-packet", "version": "0.1.0"},
                        }
                elif method == "ping":
                    response["result"] = {}
                elif not ready:
                    response["error"] = {"code": -32002, "message": "Server not initialized"}
                elif method == "tools/list":
                    response["result"] = {"tools": [TOOL]}
                elif method == "tools/call" and params.get("name") == TOOL["name"]:
                    arguments = params.get("arguments")
                    try:
                        if not isinstance(arguments, dict) or set(arguments) != {"slot", "packet_sha256"}:
                            raise KeyError
                        value = read(arguments["slot"], arguments["packet_sha256"])
                        response["result"] = {"content": [{"type": "text", "text": json.dumps(value)}]}
                    except KeyError:
                        response["result"] = {
                            "isError": True,
                            "content": [{"type": "text", "text": "Source slot is unavailable"}],
                        }
                else:
                    response["error"] = {"code": -32601, "message": "Method not found"}
        print(json.dumps(response), file=destination, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("packet", type=Path)
    parser.add_argument("admitted_sha256")
    args = parser.parse_args()
    try:
        with args.packet.open("rb") as packet:
            read = load_packet(packet.read(MAX_PACKET_BYTES + 1), args.admitted_sha256)
    except (OSError, ValueError, RecursionError):
        # Packet text and filesystem paths do not belong in startup diagnostics.
        raise SystemExit("Review packet admission failed") from None
    serve(read, sys.stdin.buffer, sys.stdout)


if __name__ == "__main__":
    main()

from io import BytesIO, StringIO
import json
from pathlib import Path
from select import select
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

from packet import load_packet
from packet_mcp import MAX_REQUEST_BYTES, serve
from test_packet import digest, packet, source


def request(identity, method, params=None):
    return {"jsonrpc": "2.0", "id": identity, "method": method, "params": params or {}}


class PacketMcpTest(unittest.TestCase):
    def setUp(self):
        self.payload = packet(source())
        self.identity = digest(self.payload)
        self.read = load_packet(self.payload, self.identity)

    def exchange(self, *requests):
        stream = BytesIO(b"".join(json.dumps(row).encode() + b"\n" for row in requests))
        output = StringIO()
        serve(self.read, stream, output)
        return [json.loads(row) for row in output.getvalue().splitlines()]

    def test_only_the_bound_tool_is_discoverable_after_initialization(self):
        pre, hello, unready, listed, unknown = self.exchange(
            request(1, "tools/list"),
            request(2, "initialize", {"protocolVersion": "2025-06-18"}),
            request(3, "tools/list"),
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            request(4, "tools/list"),
            request(5, "resources/read", {"uri": "file:///tmp/excluded"}),
        )
        self.assertEqual(pre["error"]["code"], -32002)
        self.assertEqual(hello["result"]["protocolVersion"], "2025-06-18")
        self.assertEqual(unready["error"]["code"], -32002)
        self.assertEqual([row["name"] for row in listed["result"]["tools"]], ["read_original"])
        self.assertEqual(unknown["error"]["code"], -32601)

    def test_bound_read_and_refusals_keep_separate_wire_dispositions(self):
        def call(identity, **arguments):
            return request(identity, "tools/call", {"name": "read_original", "arguments": arguments})

        hello, read, wrong, extra = self.exchange(
            request(1, "initialize", {"protocolVersion": "unknown"}),
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            call(2, slot="request", packet_sha256=self.identity),
            call(3, slot="request", packet_sha256="0" * 64),
            call(4, slot="request", packet_sha256=self.identity, path="/tmp/excluded"),
        )
        self.assertEqual(hello["result"]["protocolVersion"], "2025-11-25")
        text = read["result"]["content"][0]["text"]
        self.assertEqual(json.loads(text)["text"], source()["text"])
        self.assertTrue(wrong["result"]["isError"])
        self.assertEqual(wrong["result"], extra["result"])

    def test_oversized_frame_ends_the_exchange(self):
        output = StringIO()
        serve(self.read, BytesIO(b" " * (MAX_REQUEST_BYTES + 1)), output)
        self.assertEqual(output.getvalue(), "")

    def test_running_native_reader_keeps_captured_bytes_when_the_packet_path_changes(self):
        tool = Path(__file__).with_name("packet_mcp.py")
        replacement = packet(source(text="different captured source"))
        with TemporaryDirectory(prefix="pale-packet-") as folder:
            locator = Path(folder) / "packet.json"
            successor = Path(folder) / "successor.json"
            locator.write_bytes(self.payload)
            successor.write_bytes(replacement)
            with subprocess.Popen(
                [sys.executable, "-B", str(tool), str(locator), self.identity],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            ) as server:
                def send(value):
                    server.stdin.write(json.dumps(value).encode() + b"\n")
                    server.stdin.flush()
                    if "id" not in value:
                        return None
                    self.assertTrue(select([server.stdout], [], [], 5)[0])
                    response = json.loads(server.stdout.readline())
                    self.assertEqual(response["id"], value["id"])
                    self.assertNotIn("error", response)
                    return response["result"]

                try:
                    send(request(1, "initialize", {"protocolVersion": "2025-11-25"}))
                    send({"jsonrpc": "2.0", "method": "notifications/initialized"})
                    successor.replace(locator)
                    observed = send(request(2, "tools/call", {"name": "read_original",
                        "arguments": {"slot": "request", "packet_sha256": self.identity}}))
                    captured = json.loads(observed["content"][0]["text"])
                    self.assertEqual(captured["text"], source()["text"])
                    self.assertEqual(captured["packet_sha256"], self.identity)
                    refused = send(request(3, "tools/call", {"name": "read_original",
                        "arguments": {"slot": "request", "packet_sha256": digest(replacement)}}))
                    self.assertTrue(refused["isError"])
                finally:
                    server.stdin.close()
                    try:
                        server.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        server.kill()
                        server.wait()
                        raise
                self.assertEqual(server.returncode, 0)
                self.assertEqual(server.stderr.read(), b"")

    def test_native_startup_refuses_replaced_missing_and_oversized_packet(self):
        tool = Path(__file__).with_name("packet_mcp.py")
        with TemporaryDirectory(prefix="pale-packet-") as folder:
            locator = Path(folder) / "packet.json"
            for payload in (packet(source(text="replacement")), None, b" " * (512 * 1024 + 1)):
                if payload is None:
                    locator.unlink()
                else:
                    locator.write_bytes(payload)
                observed = subprocess.run(
                    [sys.executable, "-B", str(tool), str(locator), self.identity],
                    input=b"", capture_output=True, timeout=5,
                )
                self.assertNotEqual(observed.returncode, 0)
                self.assertEqual(observed.stdout, b"")
                self.assertEqual(observed.stderr.strip(), b"Review packet admission failed")

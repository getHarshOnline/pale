from hashlib import sha256
import json
import unittest

from packet import MAX_PACKET_BYTES, load_packet


def digest(payload):
    return sha256(payload).hexdigest()


def source(slot="request", text="Résumé\n"):
    return {"slot": slot, "sha256": digest(text.encode()), "text": text}


def packet(*entries):
    return json.dumps({"revision": "review-2", "sources": entries}).encode()


class PacketTest(unittest.TestCase):
    def test_reads_the_exact_selected_bytes_with_both_identities(self):
        entry = source()
        payload = packet(entry, source("artifact"))
        identity = digest(payload)
        read = load_packet(payload, identity)
        expected = {**entry, "revision": "review-2", "packet_sha256": identity}
        result = read("request", identity)
        self.assertEqual(result, expected)
        result["text"] = "changed"
        self.assertEqual(read("request", identity), expected)

    def test_refuses_unselected_slots_paths_and_stale_packets(self):
        payload = packet(source())
        identity = digest(payload)
        read = load_packet(payload, identity)
        for slot, requested in (
            ("unselected", identity),
            ("/tmp/request", identity),
            ("../request", identity),
            ("request", digest(packet(source(text="old")))),
        ):
            with self.subTest(slot=slot, requested=requested):
                with self.assertRaisesRegex(KeyError, "Source slot is unavailable"):
                    read(slot, requested)

    def test_rejects_mutated_packet_and_source_content(self):
        payload = packet(source())
        changed = payload.replace(b"review-2", b"review-3")
        with self.assertRaisesRegex(ValueError, "admission digest"):
            load_packet(changed, digest(payload))
        changed = packet({**source(), "text": "unreviewed"})
        with self.assertRaisesRegex(ValueError, "source digest"):
            load_packet(changed, digest(changed))

    def test_rejects_duplicate_fields_and_slots(self):
        duplicate_field = b'{"revision":"r1","revision":"r2","sources":[]}'
        with self.assertRaisesRegex(ValueError, "Duplicate packet field"):
            load_packet(duplicate_field, digest(duplicate_field))
        duplicate_slot = packet(source(), source())
        with self.assertRaisesRegex(ValueError, "duplicate packet slot"):
            load_packet(duplicate_slot, digest(duplicate_slot))

    def test_rejects_a_packet_over_the_byte_ceiling(self):
        payload = b" " * (MAX_PACKET_BYTES + 1)
        with self.assertRaisesRegex(ValueError, "byte ceiling"):
            load_packet(payload, digest(payload))

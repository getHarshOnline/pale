"""Read immutable text sources from an exact, caller-admitted review packet."""

from hashlib import sha256
from hmac import compare_digest
import json
import re
from types import MappingProxyType

MAX_PACKET_BYTES = 512 * 1024
MAX_SLOTS = 32
SLOT = re.compile(r"[a-z][a-z0-9-]{0,63}", re.ASCII)
DIGEST = re.compile(r"[0-9a-f]{64}", re.ASCII)


def _unique_object(pairs):
    obj = {}
    for key, value in pairs:
        if key in obj:
            raise ValueError("Duplicate packet field")
        obj[key] = value
    return obj


def load_packet(payload, expected_digest):
    """Validate exact admitted bytes and return a read function with no path lookup."""
    if len(payload) > MAX_PACKET_BYTES:
        raise ValueError("Review packet exceeds the byte ceiling")
    if not DIGEST.fullmatch(expected_digest) or not compare_digest(
        sha256(payload).hexdigest(), expected_digest
    ):
        raise ValueError("Review packet does not match its admission digest")
    packet = json.loads(payload.decode("utf-8"), object_pairs_hook=_unique_object)
    if not isinstance(packet, dict) or set(packet) != {"revision", "sources"}:
        raise ValueError("Invalid review packet")
    revision, entries = packet["revision"], packet["sources"]
    if not isinstance(revision, str) or not SLOT.fullmatch(revision):
        raise ValueError("Invalid packet revision")
    if not isinstance(entries, list) or not 0 < len(entries) <= MAX_SLOTS:
        raise ValueError("Invalid packet source count")

    sources = {}
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != {"slot", "sha256", "text"}:
            raise ValueError("Invalid packet source")
        slot, digest, text = entry["slot"], entry["sha256"], entry["text"]
        if not isinstance(slot, str) or not SLOT.fullmatch(slot) or slot in sources:
            raise ValueError("Invalid or duplicate packet slot")
        if not isinstance(text, str) or not isinstance(digest, str):
            raise ValueError("Invalid packet source content")
        if not DIGEST.fullmatch(digest) or not compare_digest(
            sha256(text.encode("utf-8")).hexdigest(), digest
        ):
            raise ValueError("Packet source digest mismatch")
        sources[slot] = (digest, text)
    sealed = MappingProxyType(sources)

    def read(slot, packet_digest):
        if (
            not isinstance(packet_digest, str)
            or not DIGEST.fullmatch(packet_digest)
            or not compare_digest(packet_digest, expected_digest)
            or not isinstance(slot, str)
            or slot not in sealed
        ):
            raise KeyError("Source slot is unavailable")
        digest, text = sealed[slot]
        return {
            "revision": revision,
            "packet_sha256": expected_digest,
            "slot": slot,
            "sha256": digest,
            "text": text,
        }

    return read

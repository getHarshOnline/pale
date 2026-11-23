# Read a sealed review packet

`packet.py` validates a UTF-8 JSON packet against the digest admitted by
the caller. Supply that digest through the trusted binding. The helper
does not establish the caller's authority or prepare the packet.

The packet has a `revision` and a `sources` array. Each source has a
`slot`, the source's `sha256`, and its exact UTF-8 `text`. Use stable
lowercase slot names such as `request` and `artifact`. The helper checks
each source digest, rejects duplicate fields and slots, and limits the
packet to 512 KiB and 32 sources. This format carries captured text.
Use the native evidence route for binary or live sources.

Load the bytes with `load_packet(payload, expected_digest)`. Expose its
returned function through the owner's admitted read tool. Call it with
`read(slot, packet_digest)`, where the reviewer supplies the expected
packet digest. The result includes the exact text, both digests, the
revision, and the slot. Unknown slots and stale packet digests return
the same refusal. A slot never resolves to a path or triggers a file
read. Retire the reader when the admitted packet changes.

For a local MCP host, register `python3 -B /path/to/packet_mcp.py
/path/to/packet.json ADMITTED_SHA256` as the command and its arguments.
Use the owner-selected packet path and trusted digest at startup. The
adapter offers only `read_original` over stdio. The client supplies
`slot` and `packet_sha256` on each call. The protocol carries no source
locator field. Replacing the packet file does not change a running reader.
Start a new reader with the newly admitted digest and retire the old
one before asking for a new review.

Use a disposable host configuration to inspect the complete advertised
tool inventory. The reader limits its own responses. Native host
capabilities outside this tool require their own confinement checks.

The native caller owns capturing original bytes, revision and authority
checks, permission to disclose each selected source, access to the
read tool and its lifecycle. Retain the admission and lookup evidence
separately. A captured read establishes those bytes at their captured
revision. Live source verification needs its own observation.

Check the maintained reader and its native process from the repository root:

```sh
python3 -B -m unittest discover -s tools/review-verification -p 'test_*.py'
```

The tests exercise captured reads after a packet path is replaced, stale-digest
refusal and fresh-process refusal of replaced, missing or oversized input.
They require no model provider or historical qualification directory. They do
not establish a host's wider confinement or the authority to disclose a packet.

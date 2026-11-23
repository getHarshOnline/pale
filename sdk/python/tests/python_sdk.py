"""Exercise mixed-package typing and the confined Python builder."""

from __future__ import annotations

import ast
import builtins
import json
import os
import pathlib
import socket
from unittest.mock import patch

import pypale
from pydantic import ValidationError


def candidate() -> pypale.WorkspaceModulePreimage:
    """Return the smallest valid, secret-free module preimage."""

    return pypale.WorkspaceModulePreimage(
        module_id="example",
        release_id="release-1",
        publisher=pypale.WorkspacePublisherRef(
            publisher_id="example-publisher",
            root_key_id="example-root",
            delegated_release_key_id="example-release",
            recovery_authority_id="example-recovery",
        ),
    )


module = candidate()
try:
    module.module_id = "mutated"  # type: ignore[misc]
except ValidationError:
    pass
else:
    raise AssertionError("builder models must be frozen")

try:
    pypale.WorkspaceModulePreimage.model_validate(
        {
            **module.model_dump(mode="python"),
            "ambient_path": "/private/workspace",
        },
        strict=True,
    )
except ValidationError:
    pass
else:
    raise AssertionError("builder models must reject unknown fields")

client = pypale.Pale()
builder = pypale.WorkspaceModuleBuilder(client)
with (
    patch.object(builtins, "open", side_effect=AssertionError("ambient file access")),
    patch.object(os, "getenv", side_effect=AssertionError("ambient environment access")),
    patch.object(socket, "socket", side_effect=AssertionError("ambient network access")),
):
    built = builder.build(module)

value = json.loads(built.canonical_bytes)
assert built.family == "workspace-module.v1"
assert built.value.schema_id == "pale/workspace-module.v1"
assert built.value.module == module
assert built.value.module_digest == value["module_digest"]
assert len(built.value.module_digest) == 64
assert value["schema"] == "pale/workspace-module.v1"
assert value["schema_version"] == 1
assert value["module"] == module.model_dump(mode="json", by_alias=True, exclude_none=True)
assert built.schema_sha256
assert builder.build(module).canonical_bytes == built.canonical_bytes
assert pypale.build_workspace_module(module, client=client) == built

assert (pathlib.Path(pypale.__path__[0]) / "py.typed").is_file()
native_stub = pathlib.Path(pypale.__path__[0]) / "_native.pyi"
ast.parse(native_stub.read_text(encoding="utf-8"), filename=str(native_stub))
print(json.dumps({"module_digest": built.value.module_digest, "capabilities": len(client.capability_ids)}, indent=2))

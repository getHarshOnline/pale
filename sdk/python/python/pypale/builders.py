"""Pure Python ergonomics over the API-owned workspace-module constructor."""

from __future__ import annotations

import json
from typing import Optional

from ._native import Pale
from .models import BuiltWorkspaceModule, WorkspaceModule, WorkspaceModulePreimage


class WorkspaceModuleBuilder:
    """Build canonical modules without acquiring ambient host authority.

    The Python layer serializes an owned Pydantic value only as transport input.
    It neither computes a digest nor claims canonical JSON. The native API
    performs the authoritative typed sealing, schema validation, and RFC 8785
    encoding before this method constructs the immutable result view.
    """

    __slots__ = ("_client",)

    def __init__(self, client: Optional[Pale] = None) -> None:
        self._client = client if client is not None else Pale()

    def build(self, candidate: WorkspaceModulePreimage) -> BuiltWorkspaceModule:
        """Seal one owned candidate through the Rust construction kernel."""

        transport = json.dumps(
            candidate.model_dump(mode="json", by_alias=True, exclude_none=True),
            ensure_ascii=False,
            allow_nan=False,
            separators=(",", ":"),
        ).encode("utf-8")
        document = self._client.build_workspace_module(transport)
        value = WorkspaceModule.model_validate_json(document.canonical_bytes, strict=True)
        return BuiltWorkspaceModule(
            value=value,
            schema_sha256=document.schema_sha256,
            canonical_bytes=document.canonical_bytes,
        )


def build_workspace_module(
    candidate: WorkspaceModulePreimage,
    *,
    client: Optional[Pale] = None,
) -> BuiltWorkspaceModule:
    """Build one module with an optional caller-owned native client."""

    return WorkspaceModuleBuilder(client).build(candidate)


__all__ = ["WorkspaceModuleBuilder", "build_workspace_module"]

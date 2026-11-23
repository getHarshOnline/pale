# PyPale

PyPale (`pypale`) is the Python SDK. Use it to compile workspace configuration, inspect a profile, or review a proposed change. It calls the canonical Rust SDK and returns Python values, typed errors, and handles to validated state. The Pale application implements the behavior; Python supplies the language interface.

The public import is `pypale`; the compiled module remains private as `pypale._native`.

## Build a local wheel

Use Python 3.9 or later and Rust 1.94 or later. Activate an isolated Python environment, then work from `sdk/python`.

::::steps

:::step "Build the wheel"

```sh
python -m pip wheel --no-deps . --wheel-dir dist --config-settings="maturin.build-args=--profile release --locked"
```

:::

:::step "Install that wheel"

Use the exact filename printed by the build:

```sh
python -m pip install "/absolute/path/to/pypale-0.0.1-<wheel-tags>.whl"
```

:::

::::

> [!IMPORTANT]
>
> Use the normal Python backend above. Direct `maturin build` produces an intermediate wheel without the final constructor typing. [Packaging details](#packaging-details) explains the difference for SDK maintainers.

The build and installation may fetch dependencies. Keep the same Python environment for the examples below.

## Compile a workspace from memory

An editor or build tool can supply the documents it already holds. Pass the logical root label, such as `pale-workspace.toml`, and every selected document as `(label, bytes)`. Include the root itself and its selected modules and locks. Pale resolves those supplied documents without opening their labels as files.

After installing the local wheel, save this as `compile_example.py` in `sdk/python` and run `python compile_example.py`. It loads the [complete workspace example](../../docs/workspace.md#try-a-complete-portable-workspace):

```python
from pathlib import Path
from pypale import Pale

folder = Path(__file__).resolve().parents[2] / "docs/examples/workspace"
labels = ("pale-workspace.toml", "resolution.lock.toml", "universe.lock.toml")
documents = [(label, (folder / label).read_bytes()) for label in labels]

api = Pale()
workspace = api.compile_portable_workspace("pale-workspace.toml", documents)
print(f"{workspace.workspace_id}: {workspace.source_document_count} documents")
```

Expect `notes-workspace: 3 documents`. The returned `workspace` is the compilation to inspect or compare with a separately compiled candidate. In an editor, use its buffers in place of this file loader. A missing source or invalid relationship raises `PaleDiagnosticError` instead of returning a partly compiled workspace. The handle keeps its validated state available while you query it.

Portable compilation checks the workspace description. It does not load Package implementations or generate repository guidance. The [workspace contract](../../docs/workspace.md) explains the complete configuration selection; [Inspect your workspace](https://rawthoughts.in/pale/inspect-your-workspace/) shows the corresponding command-line task.

## Inspect a contract

This example checks which API capabilities the installed wheel exposes, reads the exact embedded contract-bundle identity, retrieves one generated schema, and handles a rejected lookup.

```python
from pypale import Pale, PaleDiagnosticError

api = Pale()

print(f"Python can call {len(api.capability_ids)} Pale capabilities")

bundle = api.contract_bundle_identity()
print(f"contract members: {bundle.member_count}")
print(f"schema set: {bundle.schema_set_sha256}")

diagnostic_schema = api.contract_schema("diagnostic.v1")
print(diagnostic_schema.schema_uri)
print(f"schema bytes: {len(diagnostic_schema.bytes)}")

try:
    api.contract_schema("not-a-selected-schema")
except PaleDiagnosticError as error:
    print(error.diagnostic.code)
    print(error)
```

`Pale()` verifies the embedded contract bundle when it constructs the facade. It does not start a daemon, activate a generation, or perform a native effect. `capability_ids` lists the operations exposed by this Python binding. It does not check whether configured machines, providers or a resident are available.

`contract_schema()` returns metadata and the exact generated schema bytes from the Rust contract bundle. An API-reported failure raises `PaleDiagnosticError`. Use `error.diagnostic` for the typed view and `error.diagnostic_bytes` when you need the exact canonical diagnostic bytes. Python argument errors can raise `TypeError`; reuse of a consumed review stage raises `ValueError`.

## Compilation and configuration review

For the exact available methods and signatures in your installed wheel, use Python's generated help:

```sh
python -c 'from pypale import Pale; help(Pale)'
```

The wheel also ships `pypale/_native.pyi` and `py.typed` for editor navigation. These describe this build; there is no separately published complete Python method reference on this documentation site.

### Choose a compilation or review operation

- `compile_portable_workspace()` compiles an explicit batch of labelled source bytes. It does not scan the current directory.
- Effective-profile methods keep declared intent, the portable effective profile, and machine realization separate.
- `inspect_local_control_store()` classifies one principal or deployment store without creating, repairing, or admitting it. `Ready` is a physical state, not writer or resident authority.
- Configuration improvement uses phase-typed handles for inspect, explain, propose, evaluate, diagnose, and preview. The prepared route reuses an admitted candidate. A failed preparation is retryable, while commit requires the same explanation handle that prepared the candidate, not another explanation with equal contents.
- `preview_local_configuration_improvement(LocalConfigurationImprovementRequest(...))` captures a baseline and candidate workspace and returns the same preview handle. The request selects `baseline_manifest`, `candidate_manifest`, `project_id`, `execution_workspace_id`, and `machine_id`. Only the baseline supplies Package content; the candidate needs no Package installation.

Configuration-improvement stages are linear. A consuming transition makes the prior stage unavailable. A preview can end as accepted, rejected, cancelled, or stale. Acceptance performs no source mutation and grants no authority. `mark_stale()` records only drift that the Python caller has independently proved.

### Keep handles alive

Other journeys also return handles to Rust-owned admitted state. Python cannot fabricate the state behind a handle. A handle retains the state it needs even after the originating `Pale` object is released. Do not reuse a handle after a consuming transition.

### Page subjects and dependencies

`WorkspaceComparison.subjects(limit, continuation=None)` discovers full subject identities from either retained input, including predecessor-only subjects. `PortableWorkspaceCompilation.direct_dependencies(subject, limit, continuation=None)` returns the selected root and its direct proof inputs, including typed facts, rules and provenance. Cursors bind the exact owners and raw limit; dependency cursors also bind the full subject. Zero selects 64 rows and values above 256 cap at 256, within Python's unsigned-16-bit argument range. These bounds do not limit total bytes or retained memory, and neither query grants recipient access.

The package also provides a strict frozen Pydantic workspace-module builder. The builder improves Python ergonomics only. Rust owns typed sealing, generated schema validation, the module digest, and canonical JSON.

### Run native operations from the bound host

`compiled_interface_identity()` reports compiled SDK version, interface and target claims. `verify_current_executable_binding(compilation)` checks the current Python executable against the captured workspace binding, not the loaded extension's bytes. Neither call observes a resident or establishes process health. Local settlement checks this same binding before making changes. A workspace bound to a separate Pale CLI executable cannot be settled directly from Python.

## ABI and typing maintenance

The following sections are for changing and testing PyPale itself. Applications using a built wheel can return to the [workspace contract](../../docs/workspace.md) or [choose an interface](../../docs/interfaces.md).

### ABI and typing

`pyproject.toml` owns the Python build and dependency requirements. `uv.lock` freezes the conformance environment without narrowing the public Pydantic compatibility range.

The wheel includes `py.typed` and native type stubs generated from collected PyO3 declarations. Factory-only classes get a required positional-only `NoReturn` constructor argument before rendering. This prevents ordinary typed direct construction while preserving real constructors, named factories and enum members. Runtime rejection remains PyO3-owned; `Any`, casts and ignored errors are not security boundaries. Unqualified inherited construction fails the build instead of being guessed. Unknown module exports remain unknown, including the external diagnostic exception. The small public stub adds only that exception's typed attributes and explicit builder reexports. PyO3's `create_exception!` does not emit that class stub. Replacing the exception with a Python subclass would raise the selected ABI's Python floor.

Cargo tests verify translation laws. The native acceptance gate must also build and install the mixed CPython wheel in an isolated environment, then run `tests/native_abi.py` against the shared conformance fixtures. Importing a renamed Cargo library file is not equivalent ABI evidence.

From `sdk/python`, with the installed wheel's environment active, run:

```sh
python tests/native_abi.py tests/fixtures/cross-leaf.v1
```

After installing the wheel, run the strict public-consumer check with the pinned checker:

```sh
python tests/typecheck.py --pyright <Pyright-1.1.408-executable>
python tests/factory_constructors.py --pyright <Pyright-1.1.408-executable>
```

That check must accept the valid consumer and reject incorrect field types, unknown attributes, and wrong improvement phases. A syntax-only stub check is not a substitute.

## Packaging details

The backend delegates compilation and metadata to pinned Maturin 1.15.0. A build-only tool reads the exact extension's PyO3 metadata with pinned `pyo3-introspection` 0.29.0, adds unavailable constructors to the collected class model, and uses its existing stub renderer. Pinned wheel 0.45.1 owns final unpacking, checksum verification and repacking. All other wheel member bytes, including the extension, metadata and licences, must remain unchanged. Direct `maturin build` output is an intermediate package: it bypasses this constructor-typing correction and is not the final SDK wheel.

PEP 660 editable installation (`python -m pip install -e .`) uses the same backend. It generates `python/pypale/_native.pyi` from the extension Maturin installs there. Those are generated files, not authored stubs. As with Maturin, an editable build updates the source extension before all later checks finish; a failed build returns no wheel but does not restore an older extension/stub pair. Use a disposable checkout when testing this route. A PEP 517 source build (for example `python -m build --sdist`, with the `build` frontend installed) includes the backend and locked stub tool. This checkout resolves unpublished Rust `0.0.1` dependencies through the root `.cargo/config.toml`. Building the unmodified source archive currently requires those same explicit Cargo patches to an available dependency checkout; the archive does not bundle that checkout and is not a standalone public distribution. Once dependencies resolve, it uses the same final-wheel route. Neither path uses a stale generated stub as fallback.

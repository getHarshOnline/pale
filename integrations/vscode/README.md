# Pale for Visual Studio Code

Use this extension to edit Pale Workspace configuration, inspect the resulting profile, and review a proposed change in Visual Studio Code. It starts or connects to `pale-lsp` and presents standard language features plus the Pale Profile Explorer. The language server and compiler remain the owners of configuration meaning and diagnostics.

## Run the extension locally

Use a matching Pale source checkout, Rust 1.94 or later, Node.js 22 or later, and VS Code 1.96 or later. This is a source-development setup, not a published extension install. It lets you edit the small portable workspace below; a full machine-bound profile needs its own configuration.

From the repository root, build the language server:

```sh
cargo build --locked --manifest-path interfaces/lsp/Cargo.toml
```

With the checkout's default Cargo target directory, the executable is `target/debug/pale-lsp`. If you chose a different target directory, use that build's actual absolute path in the settings below.

The extension also needs the matching local TypeScript contracts. From `sdk/js/contracts`, with its pinned build dependencies available, build and pack it into an existing artifact directory:

```sh
npm run build
npm pack --ignore-scripts --pack-destination /absolute/path/editor-artifacts
```

Then run from `integrations/vscode`:

```sh
npm install --ignore-scripts --no-save /absolute/path/editor-artifacts/rtepl-pale-contracts-0.0.1.tgz
npm run build
```

The local archive supplies the private contracts package; npm can fetch the extension's other dependencies. `--no-save` leaves the owning package and lock unchanged. The [contracts manual](../../sdk/js/contracts/README.md) describes that package's separate build and validation behavior.

## Configure an editor workspace

Create a new folder and copy the three files from the [portable workspace example](../../docs/workspace.md#try-a-complete-portable-workspace) into it: `pale-workspace.toml`, `resolution.lock.toml` and `universe.lock.toml`. Keep their names and contents together. Work on this copy rather than editing the repository's example in place.

Create `.vscode/settings.json` in that folder:

```json
{
  "pale.server.path": "/absolute/path/to/pale/target/debug/pale-lsp",
  "pale.workspace.rootManifest": "pale-workspace.toml",
  "pale.workspace.machineId": "",
  "pale.workspace.sources": [
    {
      "path": "pale-workspace.toml",
      "languageId": "pale-toml",
      "role": "workspace-manifest"
    },
    {
      "path": "resolution.lock.toml",
      "languageId": "pale-toml",
      "role": "workspace-resolution-lock"
    },
    {
      "path": "universe.lock.toml",
      "languageId": "pale-toml",
      "role": "workspace-universe-lock"
    }
  ]
}
```

Replace the server path with your actual build. Source paths are relative to the folder you open. The extension sends exactly this source list; it does not infer roles or discover the other inputs a larger workspace might need. The empty machine ID is deliberate for this portable example.

For compact Packages, register each instruction page with `"languageId": "pale-page"` and `"role": "package-page"`. Register typed data with `"languageId": "pale-toml"` and `"role": "package-data"`. The language server helps with the page’s `+++` TOML header and leaves its Markdown body alone. An incomplete header can receive completion while the compiler reports that the page is unfinished. Keep the Package manifest and the rest of its declared inputs in the same captured source set; see [Package authoring](../../docs/package-authoring.md#put-declarations-beside-the-method).

## Open the development host

With VS Code's `code` command available in your terminal, launch a separate Extension Development Host. Replace the paths with your checkout, example and new editor-profile locations:

```sh
code --new-window \
  --user-data-dir /absolute/path/pale-editor-profile \
  --extensions-dir /absolute/path/pale-editor-extensions \
  --extensionDevelopmentPath /absolute/path/to/pale/integrations/vscode \
  /absolute/path/pale-editor-example
```

Keep Workspace Trust enabled. If asked, verify that this is the example folder you created and trust it. The extension cannot run in Restricted Mode or a virtual workspace. These separate profile directories keep the development session apart from your normal editor extensions and settings.

Open `pale-workspace.toml`. Its language mode should become **Pale TOML** after the extension starts. The three unchanged source files should compile without Pale errors in **Problems**. As a visible connection check, change `schema_version = 1` to `schema_version = "one"`, wait for the diagnostic, then restore `1`. Open-buffer changes take precedence over the saved file. You can then change the project title from `Shared notes` to `Team notes` and keep working on the same source set.

If startup fails, check the configured executable, the three source paths and the **Pale Language Server** output channel. A server built from an incompatible checkout is rejected rather than silently using a different private protocol.

## Select a profile context

Run **Pale: Select Profile Context** from the Command Palette, then choose **notes** and **local**. Open **Pale Profile** in the Explorer and expand **Intended use case**. The portable example exposes that project/setting pair, but its profile sections report **No usable profile: effective-profile inspection has no selected machine identity**. That is expected: these three files describe a world without supplying a machine, Package distribution or capability bindings. It is not evidence that a running provider or repository-generation setup exists.

For an already configured workspace, set its real machine ID and list its private and Package inputs with their actual roles as well. The [language-server contract](../../interfaces/lsp/README.md#connect-an-editor) defines those roles and the supplied-source boundary. Once that profile is usable, the review commands below operate on its retained editor state.

From a selected Profile Explorer context:

1. Run **Pale: Begin Configuration Review**.
2. Edit the configured sources for that folder.
3. Run **Pale: Evaluate Configuration Candidate**.
4. Run **Pale: Inspect Configuration Preview** to read the bounded change roots, affected subjects, causal steps, and source occurrences.
5. Run **Pale: Finish Configuration Review** and choose a disposition.

Each workspace folder owns its own temporary review. The extension checks the editor, profile, instance, topology, candidate, derivation, and preview coordinates supplied by the LSP. It does not reconstruct compiler meaning. Accepting a preview records review state for a separate write gate; it does not change source. Each command is available only during its matching review phase.

## Folder ownership and synchronization

Each workspace folder owns one resource-scoped `pale-lsp` process and one ephemeral Profile Explorer context. Multi-root windows therefore preserve folder identity rather than routing every document through the first root. The extension runs in the workspace extension host (including the corresponding remote host), restarts only the folder whose relevant configuration changed, and refreshes the tree after the server publishes a canonical analysis result. It installs one exact, glob-quoted watcher per configured source in that folder; closed-file create, change, delete, and atomic-replacement events use the standard LSP watched-files route, while open editor buffers keep precedence.

The extension is disabled for untrusted and virtual workspaces because it launches a configured executable and captures explicitly selected source files. This prevents an untrusted workspace from supplying `pale.server.path`. The desktop extension host loads neither a Node nor a WebAssembly Pale compiler.

## Build and test the extension locally

```sh
npm test
```

These commands compile the extension and run its native-client boundary tests; they do not demonstrate an actual editor session or install or publish a VSIX.

The language server generates the private response types and structural checks that this extension consumes. The selected artifact covers profile contexts, profile pages and explanations, configuration review, analysis notices, and the protocol advertisement. It does not include request validators or unused response types.

The local build checks the generated inventory and exact bytes before it compiles, then copies the runtime checks and declarations into `dist`. It does not reach into the language-server repository or regenerate the artifact. Client decoders still enforce request-relative coordinates, bounds, state relationships, and independent copies. Structural validation does not admit configuration, authorize a write, or prove that a provider is running.

For command-line work on an existing configuration, return to [Inspect your workspace: Editor support](https://rawthoughts.in/pale/inspect-your-workspace/#editor-support).

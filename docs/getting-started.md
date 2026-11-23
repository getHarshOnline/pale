# Getting started

Build `pale` from the repository, then use its built-in path computation to check the executable. The command runs without a configured workspace. The same build supplies the executable for the [repository guidance walkthrough](https://rawthoughts.in/pale/materialize-repository-guidance/).

## Requirements

Use a Pale checkout and Rust 1.94 or later with Cargo. The checkout supplies the matching Pale packages through its Cargo configuration. Cargo may download other dependencies during the build. The Pale packages are currently available as source.

## Build the command-line tool

::::steps
:::step "Build from the repository root"
Run Cargo from the checkout root so it uses the checkout's dependency and target settings.

```sh
cargo build --locked --manifest-path interfaces/cli/Cargo.toml
export PALE="$PWD/target/debug/pale"
```

If your Cargo configuration uses another target directory, set `PALE` to that build's actual executable.
:::
:::step "Check the command"
```sh
"$PALE" --help
```

Look for workspace, guidance, review and action commands. Keep this terminal open so the product guides can use your `PALE` variable.
:::
:::step "Continue your task"
To use Pale, follow [Add coding instructions to a repository](https://rawthoughts.in/pale/materialize-repository-guidance/#start-with-a-guidance-workspace) or [Inspect your workspace](https://rawthoughts.in/pale/inspect-your-workspace/#check-the-selected-world). To embed the compiler, [choose an SDK](sdks.md) and start with the three-document workspace example.
:::
::::

## Run a first computation

Print the input contract for `pale.compute.path-inventory`, then pass it one path on standard input.

```sh
"$PALE" action describe pale.compute.path-inventory
printf '%s\n' \
  '{"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs"]}}' \
  | "$PALE" action pale.compute.path-inventory
```

In the JSON response, `result.output.payload.entries[0].normalized_path` should be `src/lib.rs`. Pale computes that value from the string you supplied, with no file access.

## If something goes wrong

If the command is missing, check whether the build finished and where Cargo wrote the executable. Set `PALE` to that path. If Pale rejects the JSON, read the diagnostic and compare the input with the schema from `action describe` before retrying.

Use the [CLI reference](../interfaces/cli/README.md#results-and-failures) for structured results and errors. See the [architecture](architecture.md) for the layers that handle those requests.

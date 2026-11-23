import { spawnSync } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

// Run wasm-pack with the build machine's folders mapped to fixed names, so the
// module records source locations such as `/pale/sdk/wasm/src/lib.rs` instead of
// the home folder, checkout or Cargo registry that built it. rustc prefers the
// last matching mapping, so list the broadest folder first.
const checkout = fileURLToPath(new URL("../../..", import.meta.url)).replace(
  /\/$/u,
  "",
);
const cargoHome = process.env.CARGO_HOME ?? join(homedir(), ".cargo");
const mappings = [
  [homedir(), "/home"],
  [join(cargoHome, "registry/src"), "/cargo/registry"],
  [join(cargoHome, "git/checkouts"), "/cargo/git"],
  [checkout, "/pale"],
];

const inherited = process.env.CARGO_ENCODED_RUSTFLAGS
  ? process.env.CARGO_ENCODED_RUSTFLAGS.split("\x1f")
  : (process.env.RUSTFLAGS ?? "").split(/\s+/u).filter(Boolean);
const flags = [
  ...inherited,
  ...mappings.map(([from, to]) => `--remap-path-prefix=${from}=${to}`),
];

const result = spawnSync("wasm-pack", process.argv.slice(2), {
  stdio: "inherit",
  env: { ...process.env, CARGO_ENCODED_RUSTFLAGS: flags.join("\x1f") },
});
process.exit(result.status ?? 1);

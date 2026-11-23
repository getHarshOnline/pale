import { rmSync } from "node:fs";

// wasm-pack generates this ignore file for an unpublished build directory. The
// npm package intentionally ships `dist`, so retaining it would discard the
// WebAssembly module from the package even though `package.json` includes it.
rmSync(new URL("../dist/.gitignore", import.meta.url), { force: true });

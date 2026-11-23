"""Unit tests for the native PGO evidence workflow's toolchain gate."""

import importlib.util
from pathlib import Path
import unittest


MODULE_PATH = Path(__file__).parents[1] / "benchmark_pgo.py"
SPEC = importlib.util.spec_from_file_location("benchmark_pgo", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
benchmark_pgo = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(benchmark_pgo)


class BenchmarkPgoTests(unittest.TestCase):
    def test_llvm_major_accepts_rustc_and_profdata_spellings(self) -> None:
        self.assertEqual(benchmark_pgo.llvm_major("LLVM version: 21.1.8"), 21)
        self.assertEqual(benchmark_pgo.llvm_major("LLVM version 21.1.0"), 21)

    def test_llvm_major_rejects_unversioned_output(self) -> None:
        with self.assertRaises(ValueError):
            benchmark_pgo.llvm_major("Apple clang")


if __name__ == "__main__":
    unittest.main()

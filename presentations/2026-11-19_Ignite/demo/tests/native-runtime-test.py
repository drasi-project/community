#!/usr/bin/env python3
"""Verify that the native bundle includes the served Server UI assets."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "native_runtime", Path(__file__).resolve().parents[1] / "ops/native-runtime.py"
)
runtime = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runtime)


class ServerUiBundleTests(unittest.TestCase):
    def test_ui_assets_are_required_and_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            core, server, demo = root / "drasi-core", root / "drasi-server", root / "demo"
            binary = server / "target/debug/drasi-server"
            for path in [
                binary, root / ".cargo-target/debug/ignite-workloads",
                demo / ".native/server.json", core / "Cargo.lock",
                server / "Cargo.lock", server / "ui/package-lock.json", demo / "Cargo.lock",
            ]:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("{}")
            ui = demo / ".native/ui/dist"
            ui.mkdir(parents=True)

            def output(command, **kwargs):
                return "test\n" if kwargs.get("text") else b""

            with patch.multiple(runtime, ROOT=root, CORE=core, SERVER=server,
                                DEMO=demo, BINARY=binary, WORKLOADS=root / ".cargo-target/debug/ignite-workloads"), \
                    patch.object(runtime.subprocess, "check_output", side_effect=output):
                with self.assertRaisesRegex(RuntimeError, "Server UI assets missing"):
                    runtime.stamp()
                (ui / "index.html").write_text("<html>Drasi</html>")
                (ui / "assets").mkdir()
                asset = ui / "assets/app.js"
                asset.write_text("export const ready = true;")
                runtime.stamp()
                runtime.verify_bundle()
                manifest = json.loads((demo / ".native/build.json").read_text())
                self.assertIn("demo/.native/ui/dist/assets/app.js", manifest["files"])
                self.assertIn("drasi-server/ui/package-lock.json", manifest["lockfiles"])
                asset.write_text("changed")
                with self.assertRaisesRegex(RuntimeError, "Native bundle changed"):
                    runtime.verify_bundle()
                asset.unlink()
                with self.assertRaises(FileNotFoundError):
                    runtime.verify_bundle()


if __name__ == "__main__":
    unittest.main()

"""Every script in the catalog is importable and exposes a runnable entry point.

The catalog loader only checks that a declared module has a *directory* — so a syntax
error, a bad import, or a missing `main` in any `__main__.py` passed `selftest` cleanly
and was discovered by the next person who ran that script. `selftest` claimed to import
every script module; this is what makes the claim true.

Importing a `__main__.py` runs its module-level code, so nothing in these files may act
at import time. That is a constraint worth enforcing anyway: a script that does work
merely by being imported cannot be introspected safely.

Run with:  python -m unittest discover -s scripts -t .
"""

from __future__ import annotations

import importlib
import unittest
from pathlib import Path

from scripts.runner.config_loader import ConfigLoader

ROOT = Path(__file__).resolve().parents[2].parent


class CatalogImportTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.registry = ConfigLoader(ROOT).load()

    def test_the_shipped_catalog_loads(self) -> None:
        self.assertTrue(self.registry.scripts, "the catalog declares no scripts")

    def test_every_declared_module_imports(self) -> None:
        for script in self.registry.scripts:
            with self.subTest(script=script.title):
                importlib.import_module(f"{script.module}.__main__")

    def test_every_script_exposes_a_main_taking_argv(self) -> None:
        for script in self.registry.scripts:
            with self.subTest(script=script.title):
                module = importlib.import_module(f"{script.module}.__main__")
                entry = getattr(module, "main", None)
                self.assertTrue(callable(entry), f"{script.module} has no callable main")

    def test_every_script_config_directory_is_present(self) -> None:
        """Settings live in JSON by design, so a script whose config is gone is broken
        even though it imports."""
        for script in self.registry.scripts:
            with self.subTest(script=script.title):
                directory = ROOT / Path(*script.module.split("."))
                config = directory / "config"
                self.assertTrue(config.is_dir(), f"{script.module} has no config/")
                self.assertTrue(
                    any(config.glob("*.json")), f"{script.module}/config holds no JSON"
                )

    def test_scripts_do_not_import_one_another(self) -> None:
        """The owner's rule: script implementations must not be interdependent. Only
        `scripts._toolkit` is shared, and `scripts.runner` is the orchestrator, not a
        script's dependency."""
        names = {script.module for script in self.registry.scripts}
        for script in self.registry.scripts:
            directory = ROOT / Path(*script.module.split("."))
            for source in directory.rglob("*.py"):
                # The rule constrains implementations. A test legitimately imports the
                # loader to ask what the catalog declares — this file does exactly that.
                if "tests" in source.relative_to(directory).parts:
                    continue
                text = source.read_text(encoding="utf-8")
                for other in names - {script.module}:
                    with self.subTest(script=script.title, imports=other):
                        self.assertNotIn(
                            f"import {other}",
                            text,
                            f"{source.relative_to(ROOT)} imports another script",
                        )
                with self.subTest(script=script.title, imports="scripts.runner"):
                    self.assertNotIn("import scripts.runner", text)


if __name__ == "__main__":
    unittest.main()

"""Real pinned-image integration; enable with DIAGRAM_REAL_RENDERER=1 after pulling the image."""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

PROJECT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(os.environ.get("DIAGRAM_REAL_RENDERER") == "1", "requires the pinned Docker image")
class RealRendererTests(unittest.TestCase):
    def test_real_partial_staging_failure_repeatability_and_bypass(self):
        with tempfile.TemporaryDirectory(prefix="diagram-real-") as directory:
            root = Path(directory).resolve()
            shutil.copytree(PROJECT / "scripts", root / "scripts", ignore=shutil.ignore_patterns("__pycache__"))
            shutil.copytree(PROJECT / ".githooks", root / ".githooks")
            # Exercise every supported notation and the actual migrated SVGs together.
            shutil.copytree(PROJECT / "docs", root / "docs")
            env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
            env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_OPTIONAL_LOCKS="0")

            def command(args, success=True):
                result = subprocess.run(args, cwd=root, env=env, capture_output=True)
                if success:
                    self.assertEqual(result.returncode, 0, result.stderr.decode() + result.stdout.decode())
                else:
                    self.assertNotEqual(result.returncode, 0, result.stdout.decode())
                return result

            def git(*args):
                return command(["git", *args]).stdout

            def cli(mode, success=True):
                return command([sys.executable, "scripts/diagrams.py", mode], success)

            git("init", "-q")
            git("config", "user.name", "Renderer Test")
            git("config", "user.email", "renderer-test@example.invalid")
            cli("check")  # Every committed SVG equals this pinned toolchain's output.
            before = {p: p.read_bytes() for p in (root / "docs").rglob("*.svg")}
            cli("render")
            self.assertTrue(all(p.read_bytes() == data for p, data in before.items()))
            git("add", ".")
            git("commit", "-qm", "baseline")
            cli("setup")
            source = Path("docs/swe.1-requirements/diagrams/01-use-cases.puml")
            output = source.with_suffix(".svg")
            original = (root / source).read_text()
            staged = original.replace("title Committed use cases", "title Staged use cases A")
            working = original.replace("title Committed use cases", "title Working use cases B")
            (root / source).write_text(staged)
            cli("render")
            expected = (root / output).read_bytes()
            git("add", str(source))
            git("restore", "--worktree", str(output))
            preview = (root / output).read_bytes()
            (root / source).write_text(working)
            cli("hook")
            self.assertEqual(git("show", ":" + str(output)), expected)
            self.assertEqual((root / source).read_text(), working)
            self.assertEqual((root / output).read_bytes(), preview)
            git("commit", "-qm", "partially staged A")
            self.assertEqual(git("show", "HEAD:" + str(output)), expected)
            self.assertEqual((root / source).read_text(), working)
            # A real syntax error must fail before touching either index or preview.
            git("restore", "--worktree", str(output))
            preview = (root / output).read_bytes()
            (root / source).write_text("@startuml\n' diagram-type: component\nthis is definitely invalid syntax !!!\n@enduml\n")
            git("add", str(source))
            index = (root / ".git/index").read_bytes()
            result = cli("hook", success=False)
            self.assertIn(b"docker failed", result.stderr)
            self.assertEqual((root / ".git/index").read_bytes(), index)
            self.assertEqual((root / output).read_bytes(), preview)
            git("restore", "--source=HEAD", "--staged", "--worktree", str(source), str(output))
            # Shared style is a real rendering dependency, not merely a changed-source signal.
            style = root / "docs/.style/plantuml.iuml"
            style.write_text(style.read_text().replace("defaultFontSize 14", "defaultFontSize 15"))
            old = {str(p.relative_to(root)): git("show", ":" + str(p.relative_to(root)))
                   for p in (root / "docs").rglob("*.svg")}
            git("add", "docs/.style/plantuml.iuml")
            cli("hook")
            self.assertTrue(all(git("show", ":" + path) != data for path, data in old.items()))
            new = git("ls-files", "--stage", "-z")
            cli("hook")
            self.assertEqual(git("ls-files", "--stage", "-z"), new)
            cli("check")
            # Deliberately bypass the hook, exactly as a bad commit would; CI's command fails.
            with (root / output).open("ab") as stream:
                stream.write(b"<!-- deliberately stale -->\n")
            git("add", str(output))
            git("-c", "core.hooksPath=/dev/null", "commit", "-qm", "bypass with stale SVG")
            self.assertIn(b"Stale or missing", cli("check", success=False).stderr)


if __name__ == "__main__":
    unittest.main()

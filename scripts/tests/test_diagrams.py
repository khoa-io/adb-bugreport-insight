"""Disposable-repository checks for staging and failure atomicity; no user index is touched."""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

PROJECT = Path(__file__).resolve().parents[2]
SCRIPTS = PROJECT / "scripts"
SOURCE = "docs/diagrams/a.puml"
SVG = "docs/diagrams/a.svg"
STYLE = "docs/.style/plantuml.iuml"
TEXT = "@startuml\n' diagram-type: component\n!include ../.style/plantuml.iuml\ncomponent A\n@enduml\n"
FAKE_DOCKER = '''#!/usr/bin/env python3
import hashlib,os,pathlib,sys
args=sys.argv[1:]
if os.environ.get("DIAGRAM_TEST_FAIL"):
 sys.exit("simulated missing renderer/tool")
scratch=pathlib.Path(args[args.index("--volume")+1].removesuffix(":/work"))
for arg in args:
 if arg.endswith(".puml"):
  p=scratch/arg
  text=p.read_text()
  if "INVALID" in text: sys.exit("syntax error: "+arg)
  digest=hashlib.sha256(text.encode()).hexdigest()
  p.with_suffix(".svg").write_text('<svg xmlns="http://www.w3.org/2000/svg"><text>'+digest+'</text></svg>')
'''


class StagingTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="diagram-test-")
        self.root = Path(self.directory.name) / "repo"
        self.root.mkdir()
        self.bin = Path(self.directory.name) / "bin"
        self.bin.mkdir()
        (self.bin / "docker").write_text(FAKE_DOCKER)
        (self.bin / "docker").chmod(0o755)
        self.env = dict(os.environ, PATH=str(self.bin) + os.pathsep + os.environ["PATH"])
        # Do not inherit a caller's Git index, object directory or hooks configuration.
        for key in list(self.env):
            if key.startswith("GIT_"):
                del self.env[key]
        self.env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_OPTIONAL_LOCKS="0")
        self.git("init", "-q")
        self.git("config", "user.name", "Diagram Test")
        self.git("config", "user.email", "diagram-test@example.invalid")
        (self.root / "scripts").mkdir()
        for name in ["diagrams.py", "diagram_git.py", "diagrams.lock"]:
            shutil.copyfile(SCRIPTS / name, self.root / "scripts" / name)
        (self.root / ".githooks").mkdir()
        shutil.copyfile(PROJECT / ".githooks/pre-commit", self.root / ".githooks/pre-commit")
        (self.root / ".githooks/pre-commit").chmod(0o755)
        self.write(SOURCE, TEXT)
        self.write(STYLE, "skinparam defaultFontSize 14\n")
        self.write("docs/index.md", "![Component diagram](diagrams/a.svg)\n\n[Source](diagrams/a.puml)\n")
        self.write("unrelated.txt", "base\n")
        self.write(".gitignore", "__pycache__/\n")
        self.cli("render")
        self.git("add", ".")
        self.git("commit", "-qm", "baseline")
        self.cli("setup")

    def tearDown(self):
        self.directory.cleanup()

    def command(self, args, ok=True, env=None, cwd=None):
        result = subprocess.run(args, cwd=cwd or self.root, env=env or self.env, capture_output=True)
        if ok and result.returncode:
            self.fail(result.stderr.decode() + result.stdout.decode())
        if not ok:
            self.assertNotEqual(result.returncode, 0, result.stdout.decode())
        return result

    def git(self, *args, **kwargs):
        return self.command(["git", *args], **kwargs).stdout

    def cli(self, mode, **kwargs):
        return self.command([sys.executable, "scripts/diagrams.py", mode], **kwargs)

    def write(self, path, text):
        full = self.root / path
        full.parent.mkdir(parents=True, exist_ok=True)
        full.write_text(text)

    def indexed(self, path):
        return self.git("show", ":" + path)

    def index_bytes(self):
        return (self.root / ".git/index").read_bytes()

    def test_commit_adds_svg_and_is_idempotent(self):
        self.write(SOURCE, TEXT.replace("component A", "component B"))
        self.git("add", SOURCE)
        self.git("commit", "-qm", "source changed")
        self.assertEqual(self.git("show", "HEAD:" + SVG), (self.root / SVG).read_bytes())
        self.cli("check")
        self.cli("hook")
        self.assertEqual(self.git("status", "--porcelain"), b"")

    def test_new_source_gets_generated_svg(self):
        self.write("docs/diagrams/b.puml", TEXT.replace("component A", "component New"))
        with (self.root / "docs/index.md").open("a") as stream:
            stream.write("\n![New component](diagrams/b.svg)\n\n[Source](diagrams/b.puml)\n")
        self.git("add", "docs/diagrams/b.puml", "docs/index.md")
        self.cli("hook")
        self.assertIn(b"<svg", self.indexed("docs/diagrams/b.svg"))

    def test_unstaged_tool_implementation_is_not_executed(self):
        self.write(SOURCE, TEXT.replace("component A", "component Staged"))
        self.git("add", SOURCE)
        self.write("scripts/diagram_git.py", (SCRIPTS / "diagram_git.py").read_text())
        # The launcher remains usable, but the renderer in the snapshot is the staged one.
        renderer = self.root / "scripts/diagrams.py"
        renderer.write_text(renderer.read_text().replace('config = configuration(root)\n    with',
                            'raise DiagramError("unstaged renderer must not execute")\n    with'))
        self.cli("hook")
        self.assertNotEqual(self.indexed(SVG), self.git("show", "HEAD:" + SVG))

    def test_cycle_case_collision_and_unowned_paired_output(self):
        self.write(STYLE, "!include plantuml.iuml\n")
        self.cli("render", ok=False)
        self.write(STYLE, "skinparam defaultFontSize 14\n")
        self.write("docs/diagrams/A.puml", TEXT)
        # Case-insensitive host filesystems may overwrite a.puml rather than create A.puml.
        if len(list((self.root / "docs/diagrams").glob("*.puml"))) > 1:
            self.cli("render", ok=False)
            (self.root / "docs/diagrams/A.puml").unlink()
        else:
            self.write(SOURCE, TEXT)
        self.write("docs/diagrams/new.puml", TEXT)
        self.write("docs/diagrams/new.svg", "<svg>unrelated</svg>")
        self.git("add", "docs/diagrams/new.puml", "docs/diagrams/new.svg")
        before = self.index_bytes()
        result = self.cli("hook", ok=False)
        self.assertIn(b"unrelated SVG", result.stderr)
        self.assertEqual(self.index_bytes(), before)

    def test_partial_source_and_unrelated_staging_remain_intact(self):
        self.write(SOURCE, TEXT.replace("component A", "component Staged"))
        self.git("add", SOURCE)
        staged_text = self.indexed(SOURCE)
        self.cli("render")
        expected = (self.root / SVG).read_bytes()
        self.write(SOURCE, TEXT.replace("component A", "component Working"))
        self.cli("render")
        working_svg = (self.root / SVG).read_bytes()
        self.write("unrelated.txt", "staged\n")
        self.git("add", "unrelated.txt")
        self.write("unrelated.txt", "unstaged\n")
        # A generated working preview for unstaged B is a local SVG edit: refuse safely.
        before = self.index_bytes()
        self.cli("hook", ok=False)
        self.assertEqual(self.index_bytes(), before)
        self.assertEqual((self.root / SVG).read_bytes(), working_svg)
        self.git("restore", "--worktree", SVG)
        preview = (self.root / SVG).read_bytes()
        self.cli("hook")
        self.assertEqual(self.indexed(SVG), expected)
        self.assertEqual(self.indexed(SOURCE), staged_text)
        self.assertIn("Working", (self.root / SOURCE).read_text())
        self.assertEqual((self.root / SVG).read_bytes(), preview)
        self.assertEqual(self.indexed("unrelated.txt"), b"staged\n")
        self.assertEqual((self.root / "unrelated.txt").read_text(), "unstaged\n")

    def test_staged_shared_include_and_config_are_used(self):
        before = self.indexed(SVG)
        self.write(STYLE, "skinparam defaultFontSize 18\n")
        self.git("add", STYLE)
        self.write(STYLE, "skinparam defaultFontSize 22\n")
        lock = self.root / "scripts/diagrams.lock"
        lock.write_text("invalid unstaged config\n")
        self.cli("hook")
        self.assertNotEqual(self.indexed(SVG), before)
        self.assertEqual(self.indexed(STYLE), b"skinparam defaultFontSize 18\n")
        self.assertEqual(lock.read_text(), "invalid unstaged config\n")
        self.assertEqual((self.root / SVG).read_bytes(), before)

    def test_deleted_source_removes_only_owned_svg(self):
        self.write("docs/diagrams/logo.svg", "<svg>unrelated</svg>")
        self.git("add", "docs/diagrams/logo.svg")
        self.git("rm", SOURCE)
        self.write("docs/index.md", "# No diagrams\n")
        self.git("add", "docs/index.md")
        self.cli("hook")
        self.assertNotIn(os.fsencode(SVG), self.git("ls-files", "-z"))
        self.assertFalse((self.root / SVG).exists())
        self.assertEqual((self.root / "docs/diagrams/logo.svg").read_text(), "<svg>unrelated</svg>")

    def test_rename_and_unicode_spaces(self):
        new_source = "docs/diagrams/mới name.puml"
        new_svg = "docs/diagrams/mới name.svg"
        self.git("mv", SOURCE, new_source)
        self.write("docs/index.md", "![Renamed](<diagrams/mới name.svg>)\n\n[Source](<diagrams/mới name.puml>)\n")
        self.git("add", "docs/index.md")
        self.cli("hook")
        self.assertIn(b"<svg", self.indexed(new_svg))
        self.assertNotIn(os.fsencode(SVG), self.git("ls-files", "-z"))

    def test_syntax_include_and_tool_failures_preserve_index_and_svg(self):
        for text, extra in [(TEXT + "INVALID\n", {}),
                            (TEXT.replace("plantuml.iuml", "missing.iuml"), {}),
                            (TEXT.replace("component A", "component Changed"), {"DIAGRAM_TEST_FAIL": "1"})]:
            with self.subTest(text=text):
                self.write(SOURCE, text)
                self.git("add", SOURCE)
                before, preview = self.index_bytes(), (self.root / SVG).read_bytes()
                self.cli("hook", ok=False, env=dict(self.env, **extra))
                self.assertEqual(self.index_bytes(), before)
                self.assertEqual((self.root / SVG).read_bytes(), preview)

    def test_locally_edited_svg_refuses_and_staged_edit_is_repaired(self):
        self.write(SOURCE, TEXT.replace("component A", "component Changed"))
        self.git("add", SOURCE)
        with (self.root / SVG).open("a") as stream:
            stream.write("<!-- local edit -->\n")
        before = self.index_bytes()
        self.cli("hook", ok=False)
        self.assertEqual(self.index_bytes(), before)
        self.git("add", SVG)
        self.cli("hook")
        self.assertNotIn(b"local edit", self.indexed(SVG))
        self.cli("check")

    def test_staged_svg_deletion_is_repaired(self):
        self.git("rm", SVG)
        self.cli("hook")
        self.assertEqual(self.indexed(SVG), (self.root / SVG).read_bytes())

    def test_check_detects_stale_missing_and_orphan_without_writes(self):
        original = (self.root / SVG).read_bytes()
        for mode in ["stale", "missing", "orphan"]:
            with self.subTest(mode=mode):
                (self.root / SVG).write_bytes(original)
                orphan = self.root / "docs/diagrams/orphan.svg"
                orphan.unlink(missing_ok=True)
                if mode == "stale":
                    (self.root / SVG).write_bytes(original + b"<!-- stale -->")
                elif mode == "missing":
                    (self.root / SVG).unlink()
                else:
                    orphan.write_bytes(original)
                before = self.index_bytes()
                status = self.git("status", "--porcelain")
                result = self.cli("check", ok=False)
                self.assertIn(b"render", result.stderr)
                self.assertEqual(self.git("status", "--porcelain"), status)
                self.assertEqual(self.index_bytes(), before)

    def test_unrelated_commit_does_not_need_renderer(self):
        self.write("unrelated.txt", "change\n")
        self.git("add", "unrelated.txt")
        self.git("commit", "-qm", "ordinary", env=dict(self.env, DIAGRAM_TEST_FAIL="1"))

    def test_setup_conflicts_and_uninstall(self):
        self.cli("setup")
        self.cli("uninstall")
        self.write(".git/hooks/pre-commit", "#!/bin/sh\n")
        self.cli("setup", ok=False)
        (self.root / ".git/hooks/pre-commit").unlink()
        self.git("config", "core.hooksPath", "/some/existing/hooks")
        self.cli("setup", ok=False)
        self.assertEqual(self.git("config", "--get", "core.hooksPath"), b"/some/existing/hooks\n")

    def test_alternate_index_and_linked_worktree(self):
        alternate = Path(self.directory.name) / "alternate-index"
        shutil.copyfile(self.root / ".git/index", alternate)
        environment = dict(self.env, GIT_INDEX_FILE=str(alternate))
        original = self.index_bytes()
        self.write(SOURCE, TEXT.replace("component A", "component Alternate"))
        self.git("add", SOURCE, env=environment)
        self.cli("hook", env=environment)
        self.assertEqual(self.index_bytes(), original)
        self.assertNotEqual(self.git("show", ":" + SVG, env=environment), self.indexed(SVG))
        self.git("restore", "--worktree", SOURCE, SVG)
        worktree = Path(self.directory.name) / "linked tree"
        self.git("worktree", "add", "-qb", "linked", str(worktree))
        (worktree / SOURCE).write_text(TEXT.replace("component A", "component Linked"))
        self.git("add", SOURCE, cwd=worktree)
        self.command([sys.executable, "scripts/diagrams.py", "hook"], cwd=worktree)
        self.assertIn(b"<svg", self.git("show", ":" + SVG, cwd=worktree))

    def test_index_changed_during_render_aborts(self):
        self.write(SOURCE, TEXT.replace("component A", "component Race"))
        self.git("add", SOURCE)
        fake = self.bin / "docker"
        fake.write_text(FAKE_DOCKER + '\nimport subprocess\nsubprocess.run(["git", "-C", '+repr(str(self.root))+', "add", "unrelated.txt"], check=True)\n')
        self.write("unrelated.txt", "concurrent staging\n")
        image = self.indexed(SVG)
        result = self.cli("hook", ok=False)
        self.assertIn(b"Index changed", result.stderr)
        self.assertEqual(self.indexed(SVG), image)
        self.assertEqual(self.indexed("unrelated.txt"), b"concurrent staging\n")

    def test_collision_named_outputs_remote_includes_and_unapproved_notation(self):
        for text in [TEXT.replace("@startuml", "@startuml other"),
                     TEXT.replace("plantuml.iuml", "https://example.invalid/style.iuml"),
                     TEXT.replace("component\n", "activity\n")]:
            with self.subTest(text=text):
                self.write(SOURCE, text)
                self.cli("render", ok=False)
        self.write(SOURCE, TEXT)
        self.write(SVG, "<svg>unowned</svg>")
        self.cli("render", ok=False)
        self.write("docs/index.md", "```mermaid\nflowchart TD\n```\n")
        self.cli("check", ok=False)


if __name__ == "__main__":
    unittest.main()

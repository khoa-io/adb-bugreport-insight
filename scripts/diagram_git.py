"""Index transactions for the diagram hook; never stash/reset user content."""

import os
from pathlib import Path
import sys
import tempfile

from diagrams import DiagramError, MARKER, run


def git(root, *args, **kwargs):
    return run(["git", "-C", str(root), *args], **kwargs)


def entries(data):
    result = {}
    for item in data.split(b"\0"):
        if item:
            meta, name = item.split(b"\t", 1)
            mode, oid, stage = meta.split()
            if stage != b"0":
                raise DiagramError("Resolve index conflicts before generating diagrams")
            result[os.fsdecode(name)] = (mode, oid)
    return result


def blob(root, entry):
    return git(root, "cat-file", "blob", entry[1].decode())


def tool(path):
    return path in {"scripts/diagrams.py", "scripts/diagram_git.py", "scripts/diagrams.lock",
                    ".githooks/pre-commit"}


def managed(path):
    return path.startswith("docs/") and Path(path).suffix in {".puml", ".iuml"}


def hook(root):
    raw_index = git(root, "ls-files", "--stage", "-z")
    current = entries(raw_index)
    head = {}
    try:
        tree = git(root, "ls-tree", "-r", "-z", "HEAD")
    except DiagramError:
        tree = b""  # Initial commit has no HEAD.
    for item in tree.split(b"\0"):
        if item:
            meta, name = item.split(b"\t", 1)
            mode, kind, oid = meta.split()
            if kind == b"blob":
                head[os.fsdecode(name)] = (mode, oid)
    changed = {p for p in current.keys() | head.keys() if current.get(p) != head.get(p)}
    paired = {str(Path(p).with_suffix(".svg")) for p in current.keys() | head.keys()
              if p.startswith("docs/") and p.endswith(".puml")}
    # A deleted source still owns its old output; marked orphans are also owned.
    owned = set(paired)
    for path in changed:
        if path.startswith("docs/") and path.endswith(".svg"):
            for version in (current, head):
                if path in version and blob(root, version[path]).startswith(MARKER):
                    owned.add(path)
    if not any(managed(p) or tool(p) or p in owned for p in changed):
        return  # Ordinary commits have no Docker/Java dependency.
    for path in paired:
        versions = [blob(root, version[path]) for version in (current, head) if path in version]
        if versions and not any(data.startswith(MARKER) for data in versions):
            raise DiagramError(f"{path}: paired output collides with an unrelated SVG; move it first")
    index_path = Path(os.fsdecode(git(root, "rev-parse", "--git-path", "index")).strip())
    if not index_path.is_absolute():
        index_path = root / index_path
    original_index = index_path.read_bytes()
    with tempfile.TemporaryDirectory(prefix="diagram-index-") as directory:
        snapshot = Path(directory).resolve()
        for path, entry in current.items():
            if managed(path) or tool(path) or (path.startswith("docs/") and path.endswith((".md", ".svg"))):
                if entry[0] not in {b"100644", b"100755"}:
                    raise DiagramError(f"{path}: diagram inputs must be regular Git files")
                if path in owned:
                    continue  # Output is always regenerated, including hand-edited staged images.
                target = snapshot / path
                if not target.resolve().is_relative_to(snapshot):
                    raise DiagramError(f"Unsafe index path: {path}")
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(blob(root, entry))
        for required in ("scripts/diagrams.py", "scripts/diagram_git.py", "scripts/diagrams.lock"):
            if not (snapshot / required).is_file():
                raise DiagramError(f"Stage {required} before using the diagram hook")
        # Execute the staged tooling, so unstaged tool/config changes cannot leak into SVGs.
        run([sys.executable, str(snapshot / "scripts/diagrams.py"), "_snapshot"], cwd=snapshot)
        generated = {str(p.relative_to(snapshot)): p.read_bytes()
                     for p in snapshot.rglob("*.svg") if p.read_bytes().startswith(MARKER)}
        # Existing untouched marked orphans may not be changed in this commit; collect them too.
        for path, entry in current.items():
            if path.startswith("docs/") and path.endswith(".svg") and blob(root, entry).startswith(MARKER):
                owned.add(path)
        generated = {p: data for p, data in generated.items()
                     if str(Path(p).with_suffix(".puml")) in current}
        updates = {}
        for path in owned | generated.keys():
            previous = blob(root, current[path]) if path in current else None
            data = generated.get(path)
            if data != previous:
                updates[path] = data
        # Prepare working-preview updates only when safe. Partial source/include/tool staging
        # leaves *all* working previews alone, rather than showing an older staged design.
        partial = any(not (root / p).is_file() or (root / p).is_symlink()
                      or (root / p).read_bytes() != blob(root, entry)
                      for p, entry in current.items() if managed(p) or tool(p))
        partial |= any((root / p).exists() for p in head.keys() - current.keys() if managed(p))
        previews = {}
        for path, data in updates.items():
            full = root / path
            previous = blob(root, current[path]) if path in current else None
            head_data = blob(root, head[path]) if path in head else None
            local = full.read_bytes() if full.is_file() and not full.is_symlink() else None
            if full.is_symlink() or (full.exists() and not full.is_file()):
                raise DiagramError(f"{path}: generated output conflicts with a non-regular local file")
            if local is not None and local not in (previous, head_data, data):
                raise DiagramError(f"{path}: locally edited SVG; save it elsewhere or restore it, then retry")
            if not partial and (local is not None or previous is None):
                previews[path] = data
        if git(root, "ls-files", "--stage", "-z") != raw_index:
            raise DiagramError("Index changed during diagram generation; retry the commit")
        # Hold Git's normal index lock, edit a private copy, then replace the index once.
        lock = Path(str(index_path) + ".lock")
        try:
            descriptor = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        except FileExistsError as exc:
            raise DiagramError(f"Index is locked: {lock}; finish the other Git operation and retry") from exc
        try:
            with os.fdopen(descriptor, "wb") as stream:
                stream.write(original_index)
            if index_path.read_bytes() != original_index:
                raise DiagramError("Index changed during diagram generation; retry the commit")
            records = []
            for path, data in sorted(updates.items()):
                if data is None:
                    records.append(b"0 " + b"0" * 40 + b"\t" + os.fsencode(path) + b"\0")
                else:
                    oid = git(root, "hash-object", "-w", "--stdin", input=data).strip()
                    records.append(b"100644 " + oid + b"\t" + os.fsencode(path) + b"\0")
            environment = dict(os.environ, GIT_INDEX_FILE=str(lock))
            git(root, "update-index", "-z", "--index-info", input=b"".join(records), env=environment)
            os.replace(lock, index_path)
        finally:
            lock.unlink(missing_ok=True)
        for path, data in previews.items():
            full = root / path
            if data is None:
                full.unlink(missing_ok=True)
            else:
                full.parent.mkdir(parents=True, exist_ok=True)
                with tempfile.NamedTemporaryFile(dir=full.parent, delete=False) as stream:
                    stream.write(data)
                    temporary = Path(stream.name)
                temporary.chmod(0o644)
                os.replace(temporary, full)
        print(f"diagrams: staged {sum(v is not None for v in updates.values())} SVG updates, "
              f"{sum(v is None for v in updates.values())} deletions"
              + ("; left working previews intact for partial staging" if partial else ""))


def setup(root, uninstall=False):
    existing = git(root, "config", "--get-all", "core.hooksPath").decode().splitlines() if has_hooks_path(root) else []
    if existing and existing != [".githooks"]:
        raise DiagramError(f"Existing core.hooksPath={existing!r}; integrate .githooks/pre-commit there manually")
    if uninstall:
        if existing:
            git(root, "config", "--local", "--unset", "core.hooksPath")
        print("Removed this clone's .githooks setting; inherited configuration still applies.")
        return
    default_hook = Path(os.fsdecode(git(root, "rev-parse", "--git-path", "hooks/pre-commit")).strip())
    if not default_hook.is_absolute():
        default_hook = root / default_hook
    if default_hook.exists() and not existing:
        raise DiagramError(f"Existing hook at {default_hook}; integrate it before enabling .githooks")
    hook_path = root / ".githooks/pre-commit"
    if not os.access(hook_path, os.X_OK):
        raise DiagramError(".githooks/pre-commit must be executable (chmod +x .githooks/pre-commit)")
    git(root, "config", "--local", "core.hooksPath", ".githooks")
    print("Enabled tracked pre-commit hook for this clone (and its linked worktrees).")


def has_hooks_path(root):
    try:
        git(root, "config", "--get", "core.hooksPath")
        return True
    except DiagramError:
        return False

#!/usr/bin/env python3
"""Check repository Markdown navigation and the docs presentation contract (stdlib only)."""

from collections import Counter
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]


def prose(text):
    return re.sub(r"(?ms)^([ \t]*)(`{3,}|~{3,})[^\n]*\n.*?^\1\2[ \t]*$", "", text)


def anchors(text):
    found = set(re.findall(r'<a\s+(?:[^>]*?\s)?id="([^"]+)"', text))
    counts = Counter()
    for heading in re.findall(r"(?m)^#{1,6}\s+(.+?)\s*#*\s*$", prose(text)):
        slug = re.sub(r"[^\w\- ]", "", heading.lower()).replace(" ", "-")
        found.add(slug + (f"-{counts[slug]}" if counts[slug] else ""))
        counts[slug] += 1
    return found


def main():
    # Execution evidence is maintained outside this repository. Keep checking
    # links to it when a local archive is present, but do not require that archive.
    evidence_pages = set((ROOT / "verification").rglob("*.md"))
    pages = sorted(set(ROOT.glob("*.md")) | set((ROOT / "docs").rglob("*.md"))
                   | evidence_pages | set((ROOT / ".agents").rglob("*.md"))
                   | set((ROOT / "memory-bank").rglob("*.md")))
    errors, checked, external_evidence = [], 0, 0
    cache = {p.resolve(): anchors(p.read_text(encoding="utf-8")) for p in pages}
    for page in pages:
        raw = page.read_text(encoding="utf-8")
        text = re.sub(r"(`+).*?\1", "", prose(raw))
        if page.is_relative_to(ROOT / "docs"):
            if len(raw.splitlines()) > 500:
                errors.append(f"{page.relative_to(ROOT)}: exceeds 500 lines")
            if re.search(r"(?m)^\s*```(?:rust|mermaid|plantuml|puml)\b", raw):
                errors.append(f"{page.relative_to(ROOT)}: forbidden Rust/inline-diagram code fence")
        destinations = re.findall(r"!?\[[^\n]*?\]\((<[^>]+>|[^\s)]+)(?:\s+[^)]*)?\)", text)
        destinations += re.findall(r"(?m)^\[[^\]]+\]:\s*(<[^>]+>|\S+)", text)
        for destination in destinations:
            destination = destination.strip("<>")
            if urlsplit(destination).scheme or destination.startswith("//"):
                continue
            name, _, fragment = destination.partition("#")
            target = (page.parent / unquote(name)).resolve() if name else page.resolve()
            checked += 1
            if not target.exists():
                try:
                    target.relative_to(ROOT / "verification")
                    external_evidence += 1
                    continue
                except ValueError:
                    errors.append(f"{page.relative_to(ROOT)}: missing target {destination}")
            elif fragment and target.suffix == ".md":
                if unquote(fragment) not in cache.get(target, set()):
                    errors.append(f"{page.relative_to(ROOT)}: missing anchor {destination}")
    for error in errors:
        print(error, file=sys.stderr)
    print(
        f"Checked {len(pages)} Markdown pages and {checked} local links; "
        f"{external_evidence} external verification references omitted; {len(errors)} errors."
    )
    return bool(errors)


if __name__ == "__main__":
    sys.exit(main())

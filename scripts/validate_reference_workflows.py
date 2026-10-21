#!/usr/bin/env python3
"""Run privacy-preserving user acceptance workflows against the shipped CLI.

Build first: cargo build --workspace --offline
Run: python3 scripts/validate_reference_workflows.py
Only aggregate assertions are printed; real report content stays in temporary files.
"""

import argparse
import hashlib
import json
import queue
import shutil
import subprocess
import tempfile
import threading
import sys
import zipfile
from pathlib import Path


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def fingerprint(path):
    with path.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return digest, path.stat().st_mtime_ns


class Session:
    """Real JSON-lines stdio client; each content request explicitly retries its preview."""

    def __init__(self, binary, archive):
        self.process = subprocess.Popen(
            [str(binary), "--mcp", str(archive), "--verbose"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8",
        )
        self.frames = queue.Queue()
        self.counter = 0
        self.previews = 0
        self.request_count = 0
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()
        self.errors = threading.Thread(target=self._read_errors, daemon=True)
        self.errors.start()

    def _read(self):
        try:
            for line in self.process.stdout:
                self.frames.put(json.loads(line))
        except (ValueError, UnicodeError):
            self.frames.put({"invalid_stdout": True})
        finally:
            self.frames.put(None)

    def _read_errors(self):
        for _line in self.process.stderr:
            pass

    def send(self, frame):
        self.process.stdin.write(json.dumps(frame) + "\n")
        self.process.stdin.flush()

    def request(self, method, params=None):
        self.counter += 1
        self.request_count += 1
        identifier = self.counter
        self.send({"jsonrpc": "2.0", "id": identifier, "method": method, "params": params or {}})
        while True:
            try:
                frame = self.frames.get(timeout=120)
            except queue.Empty as error:
                raise RuntimeError("MCP response timed out") from error
            require(isinstance(frame, dict) and frame.get("jsonrpc") == "2.0", "MCP stdout must contain only JSON-RPC frames")
            if frame.get("id") == identifier:
                return frame

    def initialize(self):
        frame = self.request("initialize", {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "real-case-validation", "version": "1.0.0"},
        })
        require("result" in frame, "initialize must succeed")
        require("confirm" in frame["result"]["instructions"].lower(), "initialization must explain confirmation")
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    @staticmethod
    def body(frame):
        require("result" in frame, "tool request unexpectedly used protocol error channel")
        return json.loads(frame["result"]["content"][0]["text"])

    def tool(self, name, arguments, content=True, success=True):
        params = {"name": name, "arguments": arguments}
        frame = self.request("tools/call", params)
        if content:
            challenge = self.body(frame)
            require(frame["result"].get("isError") is True, "content operation must return a preview first")
            require(challenge["error"]["code"] == "confirmation_required", "content preview code")
            data = challenge["error"]["data"]
            require(data["expires_in_seconds"] == 300 and isinstance(data.get("cost"), dict), "preview must include cost and expiry")
            self.previews += 1
            params = {"name": name, "arguments": {**arguments, "confirmation": data["confirmation_token"]}}
            frame = self.request("tools/call", params)
        if success:
            require(not frame.get("error") and not frame["result"].get("isError", False), "tool must succeed after valid retry")
            body = self.body(frame)
            require(all(key in body for key in ("truncated", "stopped_reason", "omitted_count", "notes")), "successful envelope fields")
            return body
        require("error" in frame or frame.get("result", {}).get("isError") is True, "invalid request must fail")
        return frame

    def resource(self, method, params=None, success=True):
        params = params or {}
        frame = self.request(method, params)
        require(frame.get("error", {}).get("data", {}).get("code") == "confirmation_required", "resource must preview")
        data = frame["error"]["data"]
        require(isinstance(data.get("cost"), dict) and data["expires_in_seconds"] == 300, "resource cost and expiry")
        self.previews += 1
        frame = self.request(method, {**params, "_meta": {"bugreport-insight/confirmation": data["confirmation_token"]}})
        require(("result" in frame) == success, "resource result channel")
        return frame

    def finish(self):
        self.process.stdin.close()
        try:
            code = self.process.wait(timeout=30)
            self.reader.join(timeout=5)
            self.errors.join(timeout=5)
            require(code == 0, "normal session EOF must exit zero")
            while not self.frames.empty():
                frame = self.frames.get_nowait()
                require(frame is None or "invalid_stdout" not in frame, "stdout purity at shutdown")
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
            raise RuntimeError("MCP shutdown timed out") from None

    def abort(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()


def command(binary, arguments, expected):
    result = subprocess.run([str(binary), *map(str, arguments)], capture_output=True, timeout=120)
    require(result.returncode == expected, "CLI exit code differs from contract")
    return result


def ai_workflow(binary, archive):
    session = Session(binary, archive)
    try:
        session.initialize()
        tools = session.request("tools/list")["result"]["tools"]
        expected = {"archive_info", "list_entries", "find_entries", "entry_info", "find_by_type", "read_entry", "grep_archive"}
        require(len(tools) == 7 and {tool["name"] for tool in tools} == expected, "exact seven tools")
        require(all(tool["inputSchema"]["type"] == "object" for tool in tools), "tool schemas")
        listed = session.tool("list_entries", {"limit": 10000}, content=False)
        entries = listed["data"]["entries"]
        require(len(entries) <= 200 and listed["omitted_count"] == listed["data"]["entry_count"] - len(entries), "listing cap reports omitted entries accurately")
        found = session.tool("find_entries", {"pattern": "**/*.txt", "mode": "glob", "limit": 200}, content=False)
        candidates = found["data"]["entries"]
        require(candidates, "reference archive must contain text candidates")
        # Select the main report without fixing a private device-specific filename.
        selected = max(candidates, key=lambda entry: entry["uncompressed_size"])
        entry_id = selected["entry_id"]
        # Independent local oracle: streamed ZIP bytes, never retained in logs.
        patterns = {"section_markers": b"------", "fatal_exception": b"FATAL EXCEPTION", "anr_in": b"ANR in"}
        oracle = {name: [] for name in patterns}
        source_counts = {"lines": 0, "nul_lines": 0, "nul_bytes": 0}
        with zipfile.ZipFile(archive) as zipped:
            with zipped.open(selected["path"]) as source:
                offset = 0
                for number, line in enumerate(source, start=1):
                    source_counts["lines"] += 1
                    source_counts["nul_lines"] += int(b"\x00" in line)
                    source_counts["nul_bytes"] += line.count(b"\x00")
                    for name, pattern in patterns.items():
                        if pattern in line:
                            oracle[name].append((number, offset))
                    offset += len(line)
        print("AI workflow: source oracle prepared; checking tools and search pages", file=sys.stderr)
        identity = session.tool("archive_info", {"with_sha256": True})
        require(identity["data"]["archive"]["sha256"] == fingerprint(archive)[0], "archive hash identity")
        session.tool("entry_info", {"entry_id": entry_id})
        types = session.tool("find_by_type", {"content_type": "text", "limit": 5})
        require(types["data"]["matches"], "text detection must find real files")
        first = session.tool("read_entry", {"entry_id": entry_id, "line_start": 1, "line_end": 2})
        require(first["data"]["lines"], "line read must return evidence")
        byte_args = {"entry_id": entry_id, "byte_start": 0, "byte_end": 1024}
        byte_read = session.tool("read_entry", byte_args)
        excerpt = byte_read["data"]["excerpt"]
        require(excerpt["bytes"] <= 512 and excerpt["source_bytes"] > 0, "emitted excerpt cap and source progress")
        follow_start = excerpt["source_bytes"]
        session.tool("read_entry", {"entry_id": entry_id, "byte_start": follow_start, "byte_end": follow_start + 64})
        require(session.tool("read_entry", byte_args) == byte_read, "repeat read determinism")
        query = {"entry_ids": [entry_id], "pattern": "------", "mode": "literal", "max_matches": 2, "scan_budget": 32768, "response_budget": 8192}
        page = session.tool("grep_archive", query)
        matches = list(page["data"]["matches"])
        cursor = page["data"]["next_cursor"]
        require(cursor is not None, "bounded search should require continuation on the real report")
        pages = 1
        previous_scanned = page["data"]["scan"]["total_scanned_bytes"]
        # Continue the entire main report, keeping output small and enforcing termination.
        while cursor is not None:
            page = session.tool("grep_archive", {"cursor": cursor, "max_matches": 50, "scan_budget": 33554432, "response_budget": 65536})
            pages += 1
            require(pages <= 2000, "search continuation did not terminate within bounded workflow")
            require(page["data"]["scan"]["total_scanned_bytes"] >= previous_scanned, "cumulative search progress")
            previous_scanned = page["data"]["scan"]["total_scanned_bytes"]
            matches.extend(page["data"]["matches"])
            cursor = page["data"]["next_cursor"]
        require(page["data"]["scan"]["coverage_complete"], "main report search must exhaust without gaps")
        require(matches, "section-marker query must find evidence")
        require(all(len(match["excerpt"]["text"].encode("utf-8")) <= 512 for match in matches), "search excerpts obey emitted cap")
        require([(match["line"], match["byte_offset"]) for match in matches] == oracle["section_markers"], "all section marker locators must match independent source oracle")
        triage = {}
        for name in ("fatal_exception", "anr_in"):
            answer = session.tool("grep_archive", {"entry_ids": [entry_id], "pattern": patterns[name].decode("ascii"), "mode": "literal", "scan_budget": 32768})
            actual = list(answer["data"]["matches"])
            triage_pages = 1
            while answer["data"]["next_cursor"] is not None:
                answer = session.tool("grep_archive", {"cursor": answer["data"]["next_cursor"], "scan_budget": 33554432})
                actual.extend(answer["data"]["matches"])
                triage_pages += 1
                require(triage_pages <= 2000, "triage continuation must terminate")
            require(answer["data"]["scan"]["coverage_complete"], "triage search must cover entire main report")
            require([(match["line"], match["byte_offset"]) for match in actual] == oracle[name], "triage result locators must match raw source oracle")
            triage[name] = {"matching_lines": len(actual), "pages": triage_pages, "oracle_agrees": True, "coverage_complete": True}
        locator = matches[0]
        require(all(key in locator for key in ("entry_id", "line", "byte_offset", "excerpt")), "match locator fields")
        located = session.tool("read_entry", {"entry_id": locator["entry_id"], "line_start": locator["line"], "line_end": locator["line"]})
        require(located["data"]["lines"][0]["excerpt"]["text"] == locator["excerpt"]["text"], "locator follow-up matches evidence")
        # Errors must leave the same session usable.
        session.tool("read_entry", {"entry_id": entry_id, "line_start": 0, "line_end": 1}, success=False)
        session.tool("entry_info", {"entry_id": 4294967295}, success=False)
        session.tool("grep_archive", {"pattern": "x", "mode": "regex"}, success=False)
        missing = session.tool("find_entries", {"pattern": "__validation_no_such_entry__", "mode": "literal"}, content=False)
        require(str(missing["status"]).lower() == "empty", "completed missing collection must be empty")
        recovered = session.tool("list_entries", {"limit": 1}, content=False)
        require(recovered["data"]["entries"], "session recovery after errors")
        summary = {"entries_returned": len(entries), "archive_entries": identity["data"]["archive"]["entry_count"], "text_candidates_returned": len(candidates), "search_pages": pages, "search_matches": len(matches), "search_source_bytes": previous_scanned, "coverage_complete": True, "confirmation_previews": session.previews, "json_rpc_requests": session.request_count, "source_oracle_agrees": True, "source_counts": source_counts, "triage": triage}
        session.finish()
        return summary
    finally:
        session.abort()


def engineer_and_ci(binary, archive):
    with tempfile.TemporaryDirectory(prefix="bugreport-user-validation-") as folder:
        copied = Path(folder) / "real case.zip"
        shutil.copy2(archive, copied)
        before = fingerprint(copied)
        output = copied.parent / ".real case" / "insight"
        session = Session(binary, copied)
        try:
            session.initialize()
            require(session.resource("resources/list")["result"]["resources"] == [], "missing insight must list empty")
            session.finish()
        finally:
            session.abort()
        print("Engineer workflow: generating isolated insight and checking artifacts/resources", file=sys.stderr)
        result = command(binary, ["--insight", copied], 0)
        require(result.stdout == b"", "insight stdout must be empty")
        expected = {"README.md", "system.json", "apps.json", "anr.json", "crashes.json", "memory.json", "cpu.json"}
        require({path.name for path in output.iterdir()} == expected, "exact seven insight artifacts")
        snapshots = {name: fingerprint(output / name) for name in expected}
        require(len((output / "README.md").read_text().splitlines()) <= 400, "README line cap")
        artifacts = {}
        excerpt_count = 0

        def check_excerpts(value):
            nonlocal excerpt_count
            if isinstance(value, dict):
                if isinstance(value.get("excerpt"), str):
                    excerpt_count += 1
                    require(len(value["excerpt"].encode("utf-8")) <= 512, "all artifact evidence excerpts obey emitted byte cap")
                for item in value.values():
                    check_excerpts(item)
            elif isinstance(value, list):
                for item in value:
                    check_excerpts(item)

        for name in expected - {"README.md"}:
            artifact = json.loads((output / name).read_text())
            require(isinstance(artifact, dict), "artifact must be a JSON object")
            check_excerpts(artifact)
            artifacts[name] = artifact
        require(excerpt_count > 0, "real insight must contain inspectable evidence excerpts")
        fatal_count = artifacts["crashes.json"]["counts"]["fatal_exception_hits"]
        fatal_ledger = {key: fatal_count[key] for key in ("raw_count", "matched_evidence", "suppressed_noise", "not_emitted_by_platform")}
        require(fatal_ledger["raw_count"] == fatal_ledger["matched_evidence"] + fatal_ledger["suppressed_noise"] + fatal_ledger["not_emitted_by_platform"], "fatal evidence ledger accounts for raw hits")
        incident_counts = {
            "anr_incidents": len(artifacts["anr.json"]["anr_incidents"]),
            "java_crash_incidents": len(artifacts["crashes.json"]["java_crash_incidents"]),
            "native_crash_incidents": len(artifacts["crashes.json"]["native_crash_incidents"]),
        }
        collision = command(binary, ["--insight", copied], 1)
        require(collision.stdout == b"" and collision.stderr, "collision must fail with stderr only")
        require({name: fingerprint(output / name) for name in expected} == snapshots, "collision preserves all artifact bytes and mtimes")
        session = Session(binary, copied)
        try:
            session.initialize()
            # Hash-declining tool use must not prevent later resource identity.
            session.tool("archive_info", {"with_sha256": False})
            resources = session.resource("resources/list")["result"]["resources"]
            require(len(resources) == 7 and {resource["name"] for resource in resources} == expected, "seven resources")
            total = 0
            for resource in resources:
                require(before[0] in resource["uri"], "resource identity is bound to archive SHA")
                contents = session.resource("resources/read", {"uri": resource["uri"]})["result"]["contents"]
                require(len(contents) == 1, "one whole artifact per read")
                actual = contents[0]["text"].encode("utf-8")
                require(actual == (output / resource["name"]).read_bytes(), "resource returns exact whole artifact bytes")
                total += len(actual)
            session.resource("resources/read", {"uri": "bugreport-insight://invalid/insight/README.md"}, success=False)
            require(len(session.resource("resources/list")["result"]["resources"]) == 7, "resource-error recovery")
            previews = session.previews
            session.finish()
        finally:
            session.abort()
        require({name: fingerprint(output / name) for name in expected} == snapshots, "resource reads preserve artifacts")
        require(fingerprint(copied) == before, "all copy workflows preserve input SHA and mtime")
        require(command(binary, ["--help"], 0).stdout, "help output")
        require(command(binary, [copied], 2).stderr, "bad usage diagnostic")
        missing = command(binary, ["--mcp", Path(folder) / "missing.zip"], 1)
        require(missing.stdout == b"" and missing.stderr, "missing archive fails before protocol")
        eof = subprocess.run([str(binary), "--mcp", str(copied)], input=b"", capture_output=True, timeout=30)
        require(eof.returncode == 0 and eof.stdout == b"", "EOF before initialize is success")
        return {"artifacts": 7, "resource_reads": 7, "resource_bytes": total, "confirmation_previews": previews, "collision_preserved": True, "copy_sha_mtime_unchanged": True, "cli_exit_codes_checked": [0, 1, 2], "evidence_excerpts_checked": excerpt_count, "incident_records": incident_counts, "fatal_hit_ledger": fatal_ledger}


def main():
    root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=root / "target/debug/bugreport-insight")
    parser.add_argument("--archive", type=Path, default=root / "tmp/reference-archive.zip")
    args = parser.parse_args()
    binary, archive = args.binary.resolve(), args.archive.resolve()
    before = fingerprint(archive)
    try:
        ai = ai_workflow(binary, archive)
        engineer_ci = engineer_and_ci(binary, archive)
    finally:
        require(fingerprint(archive) == before, "reference archive SHA and mtime must remain unchanged")
    print(json.dumps({"verdict": "PASS", "archive_bytes": archive.stat().st_size, "reference_sha_mtime_unchanged": True, "ai_agent": ai, "engineer_ci": engineer_ci}, indent=2))


if __name__ == "__main__":
    main()

import json
import os
import shutil
import subprocess
import tempfile
import sys
from typing import Optional, List, Dict, Any, Tuple

# Try to import FastMCP. If not installed, print a friendly error
try:
    from mcp.server.fastmcp import FastMCP
except ImportError:
    print("Error: 'mcp' package not found. Install it with: uv add 'mcp[cli]'", file=sys.stderr)
    sys.exit(1)

mcp = FastMCP("txed-tools")

TXED_BINARY = "txed"  # ensure this is in PATH or set to an absolute path


def _resolve_txed() -> Optional[str]:
    # Allows either absolute path or PATH lookup
    if os.path.isabs(TXED_BINARY):
        return TXED_BINARY if os.path.exists(TXED_BINARY) else None
    return shutil.which(TXED_BINARY)


def _run_process(argv: List[str], input_data: Optional[str]) -> Tuple[int, str, str]:
    proc = subprocess.Popen(
        argv,
        stdin=subprocess.PIPE if input_data is not None else None,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    stdout, stderr = proc.communicate(input=input_data)
    return proc.returncode, stdout, stderr


def run_txed_command(args: List[str], input_data: Optional[str] = None) -> str:
    """
    Run txed and summarize its NDJSON output for an LLM.
    Always forces JSON output and returns a human-readable summary.
    """
    txed_path = _resolve_txed()
    if not txed_path:
        return (
            f"Error: '{TXED_BINARY}' not found.\n"
            "Install txed or set TXED_BINARY to an absolute path."
        )

    # Force JSON format for reliable parsing
    # Include '--' to prevent patterns starting with '-' from being parsed as flags
    argv = [txed_path] + args + ["--format=json"]

    rc, stdout, stderr = _run_process(argv, input_data=input_data)

    modified_files: List[str] = []
    errors: List[str] = []
    non_json_lines: List[str] = []

    for line in stdout.splitlines():
        if not line.strip():
            continue

        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            non_json_lines.append(line)
            continue

        if "file" in event:
            data = event["file"]
            path = data.get("path", "<unknown>")

            if data.get("type") == "success":
                if data.get("modified"):
                    reps = data.get("replacements", 0)
                    modified_files.append(f"{path} ({reps} replacements)")

            elif data.get("type") == "error":
                msg = data.get("message", "unknown error")
                code = data.get("code")
                if code:
                    errors.append(f"{path}: {code}: {msg}")
                else:
                    errors.append(f"{path}: {msg}")

            elif data.get("type") == "skipped":
                # keep it quiet by default
                pass

        elif "run_end" in event:
            data = event["run_end"]
            pv = data.get("policy_violation")
            if pv:
                errors.append(f"Policy violation: {pv}")

    out: List[str] = []

    if modified_files:
        out.append("### Successfully modified")
        out.extend(f"- {m}" for m in modified_files)
    else:
        out.append("No files were modified")

    if errors:
        out.append("\n### Errors")
        out.extend(f"- {e}" for e in errors)

    # If txed exited nonzero but didn't emit a structured error, surface that
    if rc != 0 and not errors:
        out.append(f"\n### Exit status\n- txed exited with code {rc}")

    if non_json_lines:
        # This should not happen under --format=json, but if it does,
        # surface it as diagnostics instead of silently discarding
        out.append("\n### Diagnostics (non-JSON stdout)")
        out.extend(f"- {ln}" for ln in non_json_lines[:50])
        if len(non_json_lines) > 50:
            out.append(f"- (truncated, {len(non_json_lines) - 50} more lines)")

    if stderr.strip():
        out.append("\n### Stderr")
        out.append(stderr)

    return "\n".join(out)


@mcp.tool()
def txed_replace(
    find: str,
    replace: str,
    files: List[str],
    regex: bool = False,
    word_regexp: bool = False,
    fixed_strings: bool = False,
    dry_run: bool = False,
) -> str:
    """
    Perform a search and replace on explicit files via txed.
    """
    if fixed_strings and regex:
        # If txed supports both, you can drop this, but most tools treat these as mutually exclusive
        return "Error: 'fixed_strings' and 'regex' cannot both be true"

    args = ["--", find, replace] + files

    if fixed_strings:
        args.append("--fixed-strings")
    elif regex:
        args.append("--regex")

    if word_regexp:
        args.append("--word-regexp")

    if dry_run:
        args.append("--dry-run")

    return run_txed_command(args)


@mcp.tool()
def txed_apply(manifest: Dict[str, Any], dry_run: bool = False) -> str:
    """
    Apply a manifest describing multi-file operations.
    """
    tmp_path = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as tmp:
            json.dump(manifest, tmp)
            tmp_path = tmp.name

        args = ["apply", "--manifest", tmp_path]
        if dry_run:
            args.append("--dry-run")

        return run_txed_command(args)
    finally:
        if tmp_path:
            try:
                os.unlink(tmp_path)
            except OSError:
                pass


if __name__ == "__main__":
    mcp.run()
import json
import shutil
import subprocess
import tempfile
import sys
from typing import Optional, List, Dict, Any

# Try to import FastMCP. If not installed, print a friendly error.
try:
    from mcp.server.fastmcp import FastMCP
except ImportError:
    print("Error: 'mcp' package not found. Install it with: uv add 'mcp[cli]'", file=sys.stderr)
    sys.exit(1)

# Initialize the MCP Server
mcp = FastMCP("sd2-tools")

SD2_BINARY = "sd2"  # Ensure this is in PATH or provide absolute path

def run_sd2_command(args: List[str], input_data: Optional[str] = None) -> str:
    """
    Helper to run sd2 and parse its JSON output into a human-readable summary for the LLM.
    """
    # Force JSON format for reliable parsing
    final_args = [SD2_BINARY] + args + ["--format=json"]
    
    try:
        process = subprocess.Popen(
            final_args,
            stdin=subprocess.PIPE if input_data else None,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        stdout, stderr = process.communicate(input=input_data)
    except FileNotFoundError:
        return f"Error: '{SD2_BINARY}' binary not found. Please install sd2."

    # Parse sd2 JSON events (newline delimited)
    summary = []
    errors = []
    modified_files = []
    
    for line in stdout.splitlines():
        if not line.strip(): 
            continue
        try:
            event = json.loads(line)
            
            # Handle specific event types
            if "file" in event:
                data = event["file"]
                path = data.get("path")
                if data["type"] == "success":
                    if data.get("modified"):
                        modified_files.append(f"{path} ({data['replacements']} replacements)")
                elif data["type"] == "error":
                    errors.append(f"Error processing {path}: {data['message']}")
                elif data["type"] == "skipped":
                    # Optional: Log skipped files if verbose
                    pass
            
            elif "run_end" in event:
                data = event["run_end"]
                if data.get("policy_violation"):
                    errors.append(f"Policy Violation: {data['policy_violation']}")

        except json.JSONDecodeError:
            continue

    # Construct Final Output for the LLM
    output_msg = []
    if modified_files:
        output_msg.append("### Successfully Modified:")
        output_msg.extend([f"- {f}" for f in modified_files])
    else:
        output_msg.append("No files were modified.")

    if errors:
        output_msg.append("\n### Errors:")
        output_msg.extend([f"- {e}" for e in errors])
        
    if stderr:
        output_msg.append(f"\n### Stderr:\n{stderr}")

    return "\n".join(output_msg)


@mcp.tool()
def sd2_replace(
    find: str,
    replace: str,
    files: List[str],
    regex: bool = False,
    word_regexp: bool = False,
    fixed_strings: bool = False,
    dry_run: bool = False
) -> str:
    """
    Perform a robust search and replace on specific files.
    
    Args:
        find: The pattern to search for.
        replace: The string to replace it with.
        files: List of file paths to process.
        regex: Treat 'find' as a regular expression.
        word_regexp: Match only whole words.
        fixed_strings: Treat 'find' as a literal string (disables regex).
        dry_run: Preview changes without modifying files.
    """
    args = [find, replace] + files
    
    if regex: args.append("--regex")
    if fixed_strings: args.append("--fixed-strings")
    if word_regexp: args.append("--word-regexp")
    if dry_run: args.append("--dry-run")
    
    return run_sd2_command(args)


@mcp.tool()
def sd2_apply(manifest: Dict[str, Any], dry_run: bool = False) -> str:
    """
    Apply a complex, atomic refactoring manifest. 
    Use this for multi-file edits or when needing 'delete' operations.
    
    The manifest schema matches the sd2 JSON schema:
    {
      "files": ["path/to/file.py"],
      "transaction": "all",
      "operations": [
        { "type": "replace", "find": "foo", "with": "bar" },
        { "type": "delete", "find": "TODO: remove" }
      ]
    }
    """
    # Create a temporary file for the manifest
    with tempfile.NamedTemporaryFile(mode='w+', suffix='.json', delete=False) as tmp:
        json.dump(manifest, tmp)
        tmp_path = tmp.name

    try:
        args = ["apply", "--manifest", tmp_path]
        if dry_run:
            args.append("--dry-run")
            
        return run_sd2_command(args)
    finally:
        # Cleanup
        try:
            shutil.os.remove(tmp_path)
        except OSError:
            pass

if __name__ == "__main__":
    mcp.run()

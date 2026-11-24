# VMT tool packs — integration handoff

This additive directory is not wired into the Rust application. Foundation is a Python 3 standard-library prototype with five executable tools. No dependencies are installed automatically. Run from the repository root:

    python -m unittest discover -s tool-packs/foundation -p "test_*.py"

Send one JSON request on stdin to runner.py, e.g. {"tool":"math.calculate","args":{"operation":"add","a":2,"b":3}}. Read one JSON response on stdout. Exit 0 means success, 1 means rejected input. Manifest limits are declarations, not a sandbox.

## Codex integration
Preserve the current local working tree and all existing review, policy, journal, MCP, and skill functionality. Fetch this branch and selectively import tool-packs/ only. Do not reset, force checkout, or merge an outdated application tree. Inspect local AGENTS.md and the latest implementation first.
Use the existing registry and review broker where possible. Validate schemas and trusted pack identity before execution; never execute untrusted manifests automatically. Launch a pinned interpreter and absolute trusted script path with shell disabled, isolated environment, bounded stdin/stdout/stderr, timeout and cancellation including descendants. Treat subprocess isolation as a separate OS-enforced requirement; do not claim network/filesystem denial from manifest metadata alone. Do not log arguments or results containing private data. Return structured errors and expose only relevant tool definitions to the model.
Prefer native Rust implementations for these pure tools if that fits the existing runtime; preserve the schema and behavior and test parity.
Add registry, dispatch, rejection, cancellation, timeout, output-limit and policy tests. Test with Ollama and report actual model failures. Follow build.ps1 on Windows and existing project instructions. Do not push or merge without approval.

## Next packs (not implemented)
- Workspace: metadata, hashing, file diff, bounded tree; reuse workspace resolver and review broker.
- Repository: Git status/diff, symbol indexing, diagnostics and test adapters. Commands remain reviewed.
- Documents: Word, PDF, spreadsheet, presentation generation and rendering with output validation.
- Research: bounded HTTP retrieval, browser and citations under existing web approval.
- System: OS and dependency diagnostics; no arbitrary command auto-approval.
- Knowledge: retrieval and indexing via existing memory/RAG architecture.
- Media: FFmpeg and image processing, then desktop accessibility and computer use.

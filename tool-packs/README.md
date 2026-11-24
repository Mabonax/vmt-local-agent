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

## Standalone runtime (second increment)

Three pure packs now expose 13 tools. Run:
    
    python tool-packs/dispatch.py list
    python -B tool-packs/check_packs.py
    python -B -m unittest discover -s tool-packs -p test_runtime.py

For a call, use `python tool-packs/dispatch.py call math.calculate` and pass {"operation":"add","a":2,"b":3} on stdin.
The registry is explicitly allowlisted and validates the supported schema subset. Only trusted checked-in pure tools are eligible. Output limits are checked after buffering; this is not process isolation or a hard memory cap.

workspace_adapter.py exposes metadata, SHA-256 and bounded directory listing through inspect(root, relative, operation). Root is supplied by the trusted caller, never the model. It is not registered in the pure dispatcher. It rejects common sensitive names, traversal and links but is not a complete secret detector or race-proof sandbox. Integrate only behind the existing workspace resolver and policy broker. No writes or shell calls are provided.

## Repository intelligence
Four pure static tools: project.detect, code.metrics, code.dependencies, python.imports. Dependency extraction is limited to top-level npm, Composer and Cargo declarations; no lockfile resolution, vulnerability assessment or execution. Requires Python 3.11+ (tomllib). Git operations remain outside the pure dispatcher and must use the existing reviewed command broker.

## Office production V1
Install dependencies explicitly using `python -m pip install -r tool-packs/office/requirements.txt`. Three registered tools generate editable DOCX, XLSX and PPTX in memory and return base64 bytes (maximum 700000 raw bytes). The host must decode and save via its approved workspace artifact writer, not place base64 in model context. Spreadsheet cells are literal only; formula-like strings remain text. The scalar schema type is a VMT dispatcher extension, not standard JSON Schema. Layouts are basic fixed templates; reopen tests check structure, not visual quality. Rendering, pagination checks, formula support and template editing are future work. No automatic dependency installation at runtime.

## PDF engineering V1
Five tools generate, inspect, extract embedded text, select pages and merge small PDFs. Install the PDF requirements explicitly. Page numbers are one-based; raw PDFs are capped at 500000 bytes and 50 pages, and requests at 1 MiB. Generation supports Latin-1 text only. Extraction is not OCR. Merge/split exclude page annotations and actions but are NOT sanitizers; treat resulting files as untrusted. PDF parsing is a complex dependency boundary requiring OS-enforced isolation and hard resource limits before processing untrusted documents in production. No rendering, visual inspection, Office conversion or redaction yet.

## Artifact quality V1
Four deterministic tools inspect artifact containers, hash bytes, and check supplied rectangles for clipping or overlap. ZIP inspection checks CRC and required parts, not full OOXML validity or safety. PDF inspection checks parser readability. Geometry must be supplied by the host or renderer; intersections may be intentional and do not establish visual defects. No visual rendering, font measurement, or automatic correction is provided by this pack. Uses the existing PDF dependencies. Treat all binary parsing as an isolation boundary.

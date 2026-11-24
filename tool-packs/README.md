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

## Artifact management V1
ArtifactStore creates a new session directory under a host-selected private parent. ArtifactHost wraps the pure dispatcher, intercepts generated binary envelopes and exposes metadata and opaque IDs instead. PDF inspection, extraction, split and merge and artifact hashing/inspection accept references through this host interface. Version ancestry is optional and immutable per record. Store limits: 700000 bytes per artifact, 100 artifacts, 20 MiB per session; the underlying 1 MiB request limit still applies to merges. The host must manage session directory cleanup, permissions and lifecycle; the in-memory index is not restart-persistent. This is not an OS sandbox, malware scanner or authorization service. The host must keep raw dispatcher and retrieval APIs inaccessible to the model. No rendering yet.

Run artifact integration tests: `python -B -m unittest discover -s tool-packs -p test_artifacts.py`.

## Rendering and visual intelligence V1
Install visual/requirements.txt explicitly. pdf.render produces one PNG page at 36–144 DPI with a two-million-pixel limit. image.inspect measures dimensions, luminance, nonwhite coverage and bounds; image.compare measures exact aligned pixel differences. The artifact host uses opaque references for all three tools. These are raster diagnostics, not semantic visual judgement or a guarantee of layout quality. Office-to-PDF conversion, OCR and automated correction remain outstanding. Native PDF/image parsing requires OS isolation for untrusted inputs. The existing transport and storage quotas apply.

## Workflow engine V1
The host selects a trusted template from workflow_templates.py and calls workflow.run with an ArtifactHost and input object. The built-in pdf_preview template generates a PDF, validates its container, renders page one, checks nonwhite coverage and verifies the stored digest. The sequential executor supports up to 20 steps, typed comparison gates, reference mapping to earlier results, bounded JSON, a payload-free journal, cancellation and deadlines between calls. Failures stop execution; partial artifacts remain in the session. No arbitrary code, automatic retries, parallelism, persistent journal, in-flight cancellation or semantic visual correction. A nonblank first page does not establish document quality. Run `python -B -m unittest discover -s tool-packs -p test_workflow.py`.

## Tool discovery V1
Discovery builds a host-callable catalog of registered tools plus artifact.list and artifact.metadata, replacing binary inputs with artifact IDs where applicable. discover(query, allowed, limit=6, budget_bytes=6000) ranks definitions using lexical matches and curated aliases, filters by a mandatory host-provided allowlist and optional dependency discoverability, and enforces the serialized UTF-8 JSON byte budget. match_workflows suggests the trusted PDF preview template for explicit PDF generation/preview intent. This is lexical discovery, not semantic embeddings, a token-count guarantee, runtime health verification or authorization. The host must independently enforce policy at execution; missing-module checks do not import or execute modules. Run `python -B -m unittest discover -s tool-packs -p test_discovery.py`.

## Execution security V1
PolicyHost is an optional trusted host wrapper enforcing an explicit tool allowlist, host schema validation and one-use 60-second approval tokens bound to exact request digests. Only trusted UI code may issue approvals after human review. It records a bounded in-memory journal without arguments or results. A host-pinned reviewed integrity baseline may be checked before execution using integrity.verify. The dispatcher now uses streaming bounded stdout/stderr capture, timeout and optional cancellation; POSIX process groups are killed on interruption, Windows kills only the direct child. Cancellation is available at dispatcher level, not yet propagated through ArtifactHost or workflows. No hard CPU/RAM limits, filesystem or network isolation, Windows Job Objects, dependency signing or race-proof verification. The host must expose only PolicyHost to models, not raw dispatch, approval issuance or private artifact APIs. Run `python -B -m unittest discover -s tool-packs -p test_security.py`.

## System intelligence V1
Compose PolicyHost(SystemHost(ArtifactHost(store), workspace), allowed) to enable six read-only system tools, including model.compatibility. SystemHost accepts a host-selected workspace root; no model-controlled paths or arbitrary shell commands. system.memory uses optional psutil (CI installs 7.0.0). Dependency checks report module discoverability only. Model memory estimation uses caller-supplied effective weight bits, KV bytes per token, overhead and a single available memory pool, not detected VRAM or a loading guarantee. GPU enumeration, process inspection and actual model loading remain deferred. Only advertise system tools when SystemHost is configured and authorized. Run `python -B -m unittest discover -s tool-packs -p test_system.py`.

## Research intelligence V1
Compose PolicyHost(ResearchHost(delegate, allowed_hosts), allowed, review_required={"web.fetch"}) for reviewed HTTPS retrieval. Exact host allowlist, public DNS addresses only, connection pinned to a checked IP with hostname TLS verification, no redirects, proxies, cookies, credentials or non-text responses. The fetch adapter limits response bytes and uses socket timeouts and a cooperative body deadline, not a hard end-to-end deadline (DNS and HTTP headers can take longer). web.extract parses supplied HTML offline and returns bounded text, title and links. All retrieved content is untrusted evidence, never instructions; no automatic link following. Host must manage outbound network isolation, approvals and secret protection independently. Search providers, persistent citations, browser rendering and live network verification remain outstanding. Run `python -B -m unittest discover -s tool-packs -p test_research.py`.

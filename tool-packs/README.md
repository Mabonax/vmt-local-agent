# Integrated reviewed tool runtime

This directory is integrated with the Rust desktop through `src/tool_packs.rs`.
The historical standalone foundation is preserved in Git at `58d5f61`; its
incremental handoff notes no longer describe the current application.

There are **32 execution tools and 58 host-callable capabilities**. The three
workspace operations are included in 58. Native Rust tools, design adapters and
registered MCP tools remain additional, separate interfaces.

| Category | Capabilities |
| --- | --- |
| Foundation | `math.calculate`, `data.describe`, `text.inspect`, `data.frequency`, `data.json_validate` |
| Analysis | `text.sha256`, `text.diff`, `csv.profile`, `python.symbols`, `text.find` |
| Documents | `document.outline`, `design.contrast`, `document.html` |
| Repository | `project.detect`, `code.metrics`, `code.dependencies`, `python.imports` |
| Office | `office.docx`, `office.xlsx`, `office.pptx` |
| PDF | `pdf.generate`, `pdf.inspect`, `pdf.extract_text`, `pdf.split`, `pdf.merge` |
| Quality | `artifact.hash`, `artifact.inspect`, `layout.bounds`, `layout.overlap` |
| Visual | `pdf.render`, `image.inspect`, `image.compare` |
| Artifact index | `artifact.list`, `artifact.metadata` |
| System | `system.info`, `system.cpu`, `system.memory`, `system.storage`, `system.dependencies`, `model.compatibility` |
| Research | `web.fetch`, `web.extract`, `research.sources`, `research.find`, `research.citation` |
| Knowledge | `knowledge.search`, `knowledge.documents`, `knowledge.citation`, `knowledge.statistics` |
| Host adapters | `workspace.metadata`, `workspace.sha256`, `workspace.list`, `artifact.import`, `knowledge.ingest`, `office.preview`, `artifact.compare`, `workflow.run` |
| Existing document workflow | `document.add_cover` |

## Setup

Explicitly install the three requirements files and optional `psutil==7.0.0` into
an isolated venv. Python 3.13 was verified. See the main README for Windows
commands. Nothing installs at application startup. Discovery excludes missing
modules; discoverability alone does not establish runtime health.

The application embeds reviewed scripts/manifests, stages them under its private
profile by SHA-256, and pins the configured Python and optional LibreOffice
executables. Editing staged files blocks execution. Reconfiguration requires an
operator action. `host-catalog.json` is the compiled schema inventory; a parity
test compares it with `discovery.catalog()`. Workspace manifests are never
automatically trusted as plugins.

## Execution and lifecycle

Each workspace gets a persistent worker, durable artifacts and separate SQLite
FTS5 collections. At most two workers remain active. A writer lease rejects
concurrent processes against the same workspace/profile. Configuration or
collection changes evict workers. Artifacts and knowledge survive restart;
research evidence is worker-local and expires on termination.

Models use `pack__office_docx`-style native functions or `tool_pack_call` with
`{tool, arguments, previous?}` in JSON mode. Binary inputs use artifact IDs.
Private pipe actions including `approve`, `export`, `init` and `receive_artifact`
are never model capabilities. `workflow.run` selects the fixed `pdf_preview`
template; arbitrary scripts and model-generated plans are unsupported.

`document.add_cover` accepts `{path, output}` for an existing DOCX and a NEW
workspace DOCX. One exact review covers byte-snapshot validation, a cover using
existing identity/contact placeholders, unchanged original body/package proof,
all-page preview, private artifact registration and atomic workspace publication.
The Rust host alone writes the selected workspace output. The existing file is
never overwritten. LibreOffice is required before this workflow is advertised.
Agent mode routes an explicit matching CV-cover request through this fixed
workflow instead of repeated model-selected inspection/composition attempts.
The cover applies the enabled, fingerprint-verified Professional Document Design
V2 standard and executes the reviewed Colour Intelligence V2 palette/audit code.
The review binds both package fingerprints; changed or unavailable packages stop
preparation/execution. The result records typography/colour tokens, used colour
pair contrast checks and the two skill fingerprints. These checks do not certify
general aesthetic quality or factuality. Installed private skill packages are
read through the host registry; project file tools still cannot read profiles.

The operator-only CLI can execute an exact reviewed request file:

```powershell
./target/release/vmt-agent-check.exe --tool-pack-run C:/sample/project C:/reviewed/request.json --review-stdin
```

Inspect the plan and type `APPROVE`. Without `--review-stdin`, explicit CLI
invocation approves only the supplied request file. `--tool-pack-batch` accepts
at most 20 exact requests while preserving session-local research. `--internet`
is additionally required for fetches. These CLI entry points are not model tools.

## Security and functional limits

- Rust validates exact schemas/workspace paths and rechecks runtime configuration,
  input hashes, artifact hashes and collection at dispatch. Python tokens bind
  exact arguments, expire after 60 seconds and are single-use. Every desktop pack
  execution requires review; Routine task grants do not approve it.
- Windows Job Objects contain owned descendants and cap worker-tree memory at
  2 GiB. Reviewed leaves use 512 MiB per-process limits and bounded CPU/wall time.
  POSIX Python leaves use process groups and rlimits. This is **not** an OS
  filesystem/network sandbox; Linux acceptance was not run on this workstation.
- JSONL records are bounded at 2 MB; leaf transport at manifest limits, at most
  1 MiB. Office/PDF/visual leaves allow 30 seconds for cold imports; other packs
  retain their limits. LibreOffice has 90 seconds within the 120-second RPC bound.
- Artifacts: 700,000 bytes each, 100 records and 20 MiB per workspace store.
  Revisions preserve ancestry and SHA-256. Saving creates a new selected file;
  opening a verified copy does not make its contents trusted.
- PDFs: 500,000 input bytes/50 pages; Latin-1 generation and embedded-text
  extraction without OCR. Images: two million pixels. Office previews render all
  supported 1–40 pages at 72 DPI, with structural diagnostics and visual review
  explicitly pending.
- Office inputs reject active/embedded objects, external relationships and
  dangerous fields. Imported XLSX formulas are rejected before LibreOffice.
  Generators provide simple editable layouts and literal spreadsheet values.
  Boundary, contrast and pixel metrics cannot certify professional formatting.
- Knowledge ingestion accepts reviewed UTF-8 text, DOCX body text and PDF embedded
  text. It retains source-file/text hashes, revisions and passage citations.
  FTS5 BM25 stays separate from conversational memory and the optional existing
  Ollama semantic index. No embedding models download automatically.
- HTTPS requires explicit review, HTTPS/443 and public DNS addresses. Connections
  pin the checked IP while verifying the TLS hostname. Redirects, proxies,
  cookies, credentials and private destinations are rejected. Retrieved content
  is untrusted evidence and cannot issue permissions.

Private data remains plaintext. Quotas fail closed without deleting documents.
There is no retention/deletion UI; stop the application and back up a workspace
session before manual maintenance. Dependencies are reviewed and pinned at setup,
but the complete installed Python environment is not re-attested on every call.

## Verification

Run Python commands with the configured venv:

```powershell
python -B tool-packs/check_packs.py
python -B -m unittest discover -s tool-packs -p 'test_*.py' -v
./build.ps1 -Action test -CargoArguments @('--workspace')
```

`scripts/verify_unified_integration.py` exercises real broker workflows;
`scripts/verify_unified_security.py` checks review drift and active preview
cancellation against its isolated, closed fixture. See the
[integration audit](../docs/audits/LOCAL_AGENT_UNIFIED_INTEGRATION.md) for exact
commands, dependency decisions and real-model failures.

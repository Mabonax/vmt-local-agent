# Semantic Knowledge Retrieval and Guided Workflows

## Local semantic search (initial bounded RAG)

The agent offers `index_knowledge` and `knowledge_search` using Ollama's
local `/api/embed` endpoint. This is a real vector-similarity search over
selected project text, **not** a separate vector database, fine-tuning, or
automatic long-term learning.

Install a local embedding model using Ollama, for example:

```powershell
ollama pull nomic-embed-text
```

This workstation already has `embeddinggemma:latest`; use it without another
download. Before sending document or query text, the runtime verifies installed
local weights, the embedding capability, and absence of cloud routing metadata.
Indexing captures its loopback endpoint in the proposal and checks cancellation
between bounded batches and before persistence. An in-flight embedding request
can take up to 30 seconds to return after Stop.

In Agent mode, request:

1. "Index this project's allowed text files with nomic-embed-text."
2. Review the indexing approval. A snapshot of **up to 45 files, 100 chunks**,
   with local embeddings and plaintext excerpts, will be written outside the
   repository under `%LOCALAPPDATA%\VMT\LocalAgent\knowledge\`.
3. "Search knowledge for the controller responsible for user onboarding."
4. Inspect returned source file paths and reopen current files before edits.

Indexing is opt-in and available only when Read-only is OFF. Search is allowed
in Read-only mode. Query and documents are sent only to the configured Ollama
loopback endpoint. A model capable of embeddings must already be installed.

Current limits: 100 chunks of roughly 1,000 characters, with 120-character
overlap; 3 chunks per file, vector length up to 4,096, 8-MiB persisted index
cap, up to 5 ranked cosine-similarity results. This is intentionally small to
fit local hardware. Reindex after code changes; there is no background update.

**Privacy:** Allowed project files can still contain private data or embedded
secrets. The index stores text snippets and vectors as unencrypted JSON.
Do not index clinical, confidential, or institutional repositories without
reviewing file contents, storage requirements, and the local account's access.

## Structured development workflows

The agent offers `list_workflows`, `workflow_status`,
`workflow_start`, `workflow_advance` and `workflow_finish`.

Available types: `feature`, `bugfix`, `refactor` and `document`.
A workflow has five stages:

1. **Inspect:** Read the actual files/requirements before proposing actions.
2. **Plan:** Identify minimal scope, affected files, risks and expected results.
3. **Implement:** Make reviewed edits or use reviewed native MCP creative tools.
4. **Verify:** Run appropriate reviewed checks and inspect durable exports.
5. **Report:** Summarize verified outcomes, failures and limitations.

Every start, advance and finish operation requires review. Advancement must
include a short evidence note; this note is not independently verified by the
runtime, so approving it does not mean tests actually passed.

The workflow state persists locally per selected project, separate from the
source repository and unrelated projects. Only one active workflow per project
is supported. It is not a background agent, task scheduler or autonomous
state machine. The agent still needs to select and execute the correct tools.

## Document workflows

Built-in skills include `word-documents`, `pdf-documents`,
`spreadsheets` and `presentations`, plus `artcraft-suite`.

Recommended pattern:
- Start a `document` workflow and inspect source materials.
- Load the appropriate document skill.
- Discover a configured WordCraft, PdfCraft, GridCraft, DeckCraft or DesignCraft
  server via `mcp_servers` and `mcp_tools`.
- Request one exact MCP tool call at a time, approving operations and export
  destinations.
- Reopen or inspect the produced file through the appropriate native tool.
- Record actual verification, then advance and finish the workflow.

These skills **do not implement DOCX/PDF/XLSX/PPTX binary serialization** in
Rust. Durable generation depends on a correctly installed compatible Craft
server. A Markdown/CSV fallback can be created with the existing file tools,
but must be described as such.

## Workstation validation checklist

```powershell
.\build.ps1 -Action check
.\build.ps1 -Action test -Library
.\build.ps1 -Action clippy
.\build.ps1 -Action build -Release
```

Use a disposable repository to test fresh indexing, search results, persistence
after restart, workspace separation, read-only enforcement, missing embedding
model handling, and an updated file that requires a new index. Test each
workflow's stage advancement and deny path. Run live Craft MCP checks on a
disposable document, never a production file.

The 0.3.0 feature is unverified until Codex completes those checks.

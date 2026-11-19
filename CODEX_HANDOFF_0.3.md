# Codex handoff — VMT Local Agent 0.3.0

This branch is stacked on `feature/0.2.0-skills-memory`, itself stacked
on `fix/0.1.1-execution-reliability`. Do not merge or force-push `main`
without the user's explicit approval.

## Workstation build and corrections

```powershell
git fetch origin
git switch feature/0.3.0-docs-workflows-mcp-rag
git pull --ff-only
cargo fmt --all
.\build.ps1 -Action check
.\build.ps1 -Action test -Library
.\build.ps1 -Action clippy
.\build.ps1 -Action build -Release
```

Fix all compiler errors, Clippy warnings and test failures and then commit
formatted code to the same feature branch. Do not claim success from a compiler
check alone.

## Mandatory acceptance tests

Use disposable projects and independently check the filesystem after each
action. Do not use clinical, institutional or confidential repositories.

### Skills and workflows
- Verify `list_skills`/ `load_skill` for every built-in document skill.
- Start a feature workflow; deny advance and confirm stage unchanged.
- Approve stage transitions with evidence; restart app and confirm state.
- Finish workflow and verify no active workflow remains.
- Verify Read-only mode rejects any workflow transition.

### Local retrieval
- Install a compatible local Ollama embedding model with explicit user
  approval before attempting the index. Never download automatically.
- Approve `index_knowledge` over a small temporary project; verify local
  index exists and search returns the correct source path and snippet.
- Change source and demonstrate search is historical until reindex.
- Verify separate project indexes, missing model errors, and Read-only
  indexing denial.

### Native MCP and ArtCraft
- Review `MCP_AND_CRAFT.md` and the CLI root flags.
- Install only trusted and user-approved Craft CLIs; do not auto-download.
- Verify `mcp_servers` does not start any process.
- Verify `mcp_tools` and `mcp_call` require individual approvals.
- Deny discovery, verify no process starts.
- Discover tools from WordCraft; fetch full schema using `mcp_schema`;
  call an actual tool, then another tool during the same Agent turn;
  confirm document state survives and processes exit after the turn.
- Test PdfCraft, DeckCraft, DesignCraft, PhotoCraft and other desired apps
  individually with disposable files. Recording a registration is not
  proof of compatibility or correct export.
- Verify a nonexistent MCP tool is rejected and server failure/timeout
  is shown as failure, not successful execution.
- Verify the 4 unconfined apps remain excluded without explicit opt-in.
- Confirm approved export destination, durable file existence,
  reopen/readback and format-specific sanity checks.
- Test model-native tools with Qwen and guided edits with DeepSeek.

## Results

Write `VERIFICATION_0.3.md` containing commands actually run, models/versions,
app/CLI versions, passed and failed checks, exact failing scenarios and fixes.
Distinguish simulated unit tests from live Windows desktop acceptance.
Document unsupported MCP protocol versions or binary output features rather
than pretending the integrations work.

Commit and push the verification/fix work on the 0.3 feature branch. Keep
PR #3 in draft until accepted. Merge order is #1, then #2, then #3.

# VMT Local Agent 0.4.0 — workstation verification

Date: 19 November 2026. Scope: a Windows local development preview, using
disposable projects and public research content. This is not production,
clinical-data, adversarial sandbox, or Codex model-quality certification.

## Repository and environment

- Branch: `feature/0.4.0-policy-journal-research`, starting at
  `1e4ce81584cf7ff7554a7823c8accdf77776cd32`.
- Direct Git HTTPS/SSH connections to GitHub failed. The connected GitHub API
  retrieved the repository objects instead. All blob, tree and commit SHA-1
  identities were checked before import; `git fsck --full` passed. The complete
  71-commit history was restored; the checkout is **not shallow**. The 0.1.1,
  0.2.0 and 0.3.0 branch tips were verified as ancestors. No cumulative branches
  were merged again, no history was rewritten, and `main` stayed at `5d56e91`.
- Rust/Cargo 1.100.0, stable `x86_64-pc-windows-msvc`; Rustup 1.29.3.
- Visual Studio 2022 Community C++ tools; Windows SDK 10.0.26100.0.
  The incomplete newer VS installation was not used or reinstalled.
- PowerShell 7.6.9 host; approved command tools launch Windows PowerShell.
- Ollama 0.40.16 on loopback `127.0.0.1:11434`.
- RTX 3060 Laptop GPU, 6,144 MiB VRAM. Qwen at 8,192 context and
  embeddinggemma at 2,048 context were observed fully GPU resident together;
  total observed GPU usage was about 5,052 MiB including other desktop work.

## Findings and changes

The existing library/UI split was preserved. The pipeline remains model proposal
→ deterministic capability policy → immutable review proposal → execution broker
→ actual result → durable journal → model follow-up and runtime execution record.

1. Fixed all four initial strict-Clippy findings and formatted the repository.
2. Set package/window/protocol client version to 0.4.0. No dependency upgrade;
   the lockfile changes only the root package version.
3. Bound MCP proposals to the executable and complete flags captured before
   review. Dispatch rejects a changed registry; the UI no longer rereads a
   different configuration to describe the action.
4. Added negotiated MCP 2025-06-18 support alongside 2025-11-25, matching the
   installed EffectCraft and FigCraft servers. Added capability checking,
   bounded pagination, duplicate/schema checks, a 16-record receive queue,
   cancellable bounded stdin writes, and Windows process-tree termination.
   Oversized full schemas fail instead of silently returning partial JSON.
5. Checked embedding model local weights and embedding capability before sending
   project/query content. Cloud routing metadata is forbidden. Index proposals
   capture their loopback endpoint. Embedding responses and index writes are
   bounded; cancellation is checked between batches and before saving.
6. Recorded requested approvals in guided and normal workflows. Failed/stopped
   tasks now reach terminal journal states on normal unwinding. Missing dispatch
   results remain explicitly uncertain even when the task has a terminal state;
   no recovery path replays actions. Unknown model tool strings are not persisted
   as arbitrary journal metadata.
7. Added failure/denial counts and bounded completion corrections. A model that
   leaves failed tools unresolved receives actual workflow state and a correction;
   unresolved work is explicitly marked incomplete rather than recorded complete.
   This does not semantically verify every sentence of model prose.
8. Rejected linked ancestor directories for app persistence/skills as well as
   dedicated project files. Added a real Windows junction regression. Project
   reads now enforce their byte bound during reading; successful writes are
   read back before success is returned. Raw mutation dispatch is crate-private.
9. Applied no-proxy, public DNS/IP pinning, query validation, JSON MIME and bounded
   response handling to hosted search too; expanded special-address guards and
   bounded public URLs. Fetch provenance retains retrieval time and strips query
   strings/fragments. URL paths can still contain sensitive data: review them.
10. Added bounded retry of transient Windows local-state rename permission errors.
    Fixed repeat invocation of `build.ps1` accumulating VS environment paths until
    cmd.exe failed with `The input line is too long`.
11. Added an explicit isolated acceptance runner and updated stale documentation,
    including the README's obsolete statement that no MCP client existed.

Changed production modules: `agent`, `execution`, `knowledge`, `mcp`, `memory`,
`ollama`, `research`, `skills`, `store`, `task_journal`, `tools`, `types`,
`workspace`, `main`. Formatting also touches `policy`, `workflows`, `lib`.
Acceptance code is in `verification.rs`, `check.rs`, and
`scripts/verify-0.4.ps1`; build/version/docs changes are included separately.

## Build and automated regression checks

Required checks were run sequentially using `build.ps1` for the VS 2022 environment:

```powershell
cargo fmt --all
.\build.ps1 -Action check
.\build.ps1 -Action test -Library
.\build.ps1 -Action clippy -CargoArguments @('--all-targets','--','-D','warnings')
.\build.ps1 -Action build -Release
```

The initial branch compiled and passed 28 tests, but strict Clippy failed four
findings. The expanded suite passes **32 tests, 0 failures, 0 ignored**.
Strict all-target Clippy passes without warning suppression. Final release
artifact and formatting results are recorded in the release completion below.

Regression coverage includes read-before-edit, concurrent edits, no-clobber new
files, path/device/stream/secret guards, a real Windows junction, local endpoints,
incomplete model responses, policy/read-only gates, review-only preparation,
command failure reporting, skills limits, memory identity/reload, stripped source
metadata, bounded journal events, clean cancellation and dispatch-without-result
recovery. The interruption fixture persists a side effect, omits its result,
reloads the journal, and checks uncertainty without replay. It is not a forced
power-loss or OS crash durability test.

## Live runtime, retrieval and research

Explicit runner data lives only under `.local/verification-04`; normal user app
data, registry, chats, and existing Craft windows were preserved.

- Runtime checks passed: review preparation does not save notes or advance a
  workflow; read-only blocks writes, commands, MCP startup, indexing and notes;
  actual reviewed note persists and is isolated from another project; feature
  stage transitions and finish persist; a concurrent source edit is preserved;
  a nonzero command is a failure and a successful command captures actual output.
- A separate checker process retrieved the persisted note and uncertain journal.
  There is no automatic task replay.
- Real `embeddinggemma:latest` (`85462619ee72`, installed 621 MB model) indexed
  two synthetic source files. Password-reset/onboarding search ranked
  `accounts.rs` at **0.650** versus unrelated `plants.txt` at **0.151**.
  Changed source remained historical until reviewed reindexing, then the new
  phone-recovery snippet appeared. Separate-project, missing-model, and cloud-model
  checks failed closed. No model download was performed.
- Reviewed `https://example.com/?vmt_public_fixture=1#section` returned actual
  Example Domain text, marked untrusted. The persisted source ledger excludes
  query/fragment values and retains UTC retrieval time. Private/overlay URLs,
  URL credentials, disabled internet and credential-bearing queries were rejected.
- **Authenticated hosted web search is unverified:** `OLLAMA_API_KEY` is absent.
  No key was provisioned and no private project data was sent to a search service.
  Inference/embeddings use local Ollama; hosted search is a separate online request.

## MCP and native applications

The isolated synthetic stdio server passed paginated discovery, complete cached
schema retrieval, two-call session state, configuration-change rejection, unknown
tool rejection, unsupported-version rejection, tool-error reporting, cancellation,
oversized-record rejection, and process cleanup. It is separate from real-app proof.

| Application | Actual workstation result |
| --- | --- |
| EffectCraft 0.21.0 | MCP 2025-06-18; discovered schemas, created a 320×240 composition, saved 2,473-byte `.ecproj`, stopped server, started a fresh server, reopened and verified composition name. |
| FigCraft 0.14.0 | MCP 2025-06-18; created rectangle, edited its name, saved 3,363-byte `.figcraft`, exported 197-byte SVG, checked actual SVG markup, restarted server, reopened and verified edited name. |
| VectorCraft | CLI help inspected; MCP probe did not respond and the VMT client subsequently reported server disconnected. No compatible native export claimed. |
| CADCraft | CLI found; excluded from registration/invocation because user-wide access was not approved. |
| PhotoCraft, FilmCraft, LightCraft, PdfCraft, DesignCraft, GridCraft, WordCraft, DeckCraft, SoundCraft, MaterialCraft, TextureCraft | No matching installed CLI found in the inspected standard install locations/PATH. Native integrations unverified; no installation attempted. |

No user-wide file-access app was automatically registered. DOCX/PDF/XLSX/PPTX
creation was not verified; the successful artifacts are native EffectCraft,
native FigCraft and SVG, not substitutes claimed as office files.

## Local model evidence

| Model | Installed details | Observed isolated inference |
| --- | --- | --- |
| `qwen3.5:4b` | `d8b0f5e9760c`, 4.2B GGUF Q4_K_M | Correct 17×23 = 391; 208 output tokens, 69.45 tokens/s, 13.61 s wall time at 4,096 context. |
| `deepseek-r1-local:7b` | `0bc8d2ee64c4`, 7.6B GGUF Q3_K_M | Correct 391; 200 output tokens, 57.26 tokens/s, 9.34 s wall time at 4,096 context. |

These are single synthetic observations, including model load/other workstation
activity, not throughput benchmarks. Completion/capability metadata is not proof
of reliable agent behavior. App-level model and desktop acceptance is recorded
in the final release completion below.

The first larger Qwen run did reproduce a failing Cargo test, apply the exact
reviewed greeting change, rerun and pass the test, and save its reviewed note.
It then attempted `workflow_finish` at the plan stage, received a real failure,
and falsely claimed the workflow was complete. The checker correctly failed.
This finding prompted the runtime completion correction described above; the
original failed report is retained rather than counted as a pass.

## Remaining boundaries and release assessment

- Approval is not OS isolation. Reviewed shell/MCP processes retain Windows-user
  authority and may access resources beyond the selected project or the app's
  native network policy. CLI root flags are defense in depth, not a sandbox.
- File/link checks reduce accidents and known path attacks but do not eliminate
  TOCTOU races or protect an adversarial shared workstation. Executable bytes
  are not cryptographically pinned between approval and startup.
- App memory, conversations, indexes and provenance are plaintext. Filename and
  search-query heuristics are not comprehensive secret detection or DLP.
- Journals are atomic JSON snapshots, not SQLite WAL, cross-process transactions,
  tamper-evident audit, or exactly-once execution. Completed files accumulate;
  history examines at most 500 directory entries and returns at most 25 summaries.
  Large long-running installations need retention/indexing and concurrency work.
- Indexes are bounded historical snapshots, not current repository truth.
  Persisted corpus/model replacement does not have cryptographic identity checking.
- Stop during model transport is cooperative between streamed records with a
  240-second request limit; public fetch/search and in-flight embeddings wait for
  their bounded request to return. No instant network cancellation is claimed.
- Model prose can still be wrong, omit work or ask for an action in text instead
  of calling it. Runtime results and independent file/test checks remain decisive.
- Pending external acceptance: hosted search credentials, VectorCraft protocol
  compatibility, absent Craft CLIs and office exports; production egress/sandbox,
  privacy/legal review and adversarial testing remain out of scope.

## Evidence and reproducibility

Local reports: `.local/reports/04/` (`runtime.txt`, `resume.txt`, `rag.txt`,
`research.txt`, `craft.txt`, `model-performance.json`, model/UI acceptance files).
The ignored reports and disposable artifacts are workstation evidence, not files
silently published to GitHub. Commands can be rerun via `scripts/verify-0.4.ps1`.

Documentation fallback: Context7 was attempted twice but its MCP transport was
unavailable. Version-sensitive behavior was checked against installed crate
sources, live CLI schemas and official references:

- [MCP 2025-06-18 lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle)
- [MCP tools and pagination](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
- [Ollama embedding endpoint](https://docs.ollama.com/api/embed)

## Final release completion

- **Build: passed.** Sequential compilation, 32 library tests, all-target Clippy
  with `-D warnings`, and optimized release build passed after the fixes. The
  final release build took 3m 04s on this workstation. `cargo fmt --all -- --check`
  and normal `git diff --check` also passed. No compiler warnings were suppressed.
- **Automated tests: 32 passed, 0 failed, 0 ignored.** Deliberately failing fixture
  commands/model compatibility probes below are separate acceptance evidence.
- **Core model cases: 5 passed, 2 failed compatibility checks.** Qwen chat
  (0.5 s warm), JSON read-only file lookup (2.2 s), and native feature acceptance
  (34.6 s) passed. DeepSeek app chat (10.4 s) and packaged guided editing passed.
  DeepSeek JSON Agent mode reached its 4,096-output-token bound without a complete
  response; native Agent mode returned completion claims without inspecting the
  project. Both were rejected without executing partial/unearned actions.
- The corrected Qwen feature run performed a real failing Cargo test, one reviewed
  greeting edit, read-back, a passing test, all five workflow stages and reviewed
  memory save. It recorded 13 successful tool actions, one file change, one
  successful command, two failed attempts (including the initial test), and no
  denials. A fresh checker process independently verified source, note and history.
- The actual packaged desktop rendered with the VMT identity, discovered models,
  previewed files, selected DeepSeek, reopened a saved chat after restart, and
  preserved settings. In guided editing, independent disk reads confirmed
  `Hello world\n` while review was open, and `Hello desktop\n` only after approval.
  A subsequent denied edit preserved that file. A live longer DeepSeek chat was
  stopped with the Stop button; controls returned and an intelligible stopped
  result appeared. No freeze/deadlock was observed in these bounded checks.
- On restart into the completed Rust project, Qwen used `task_history`, reviewed
  local indexing and `knowledge_search`, returning the actual greeting sources.
  Its first memory query added unrelated words and missed the literal substring;
  a focused follow-up using `Acceptance fixture` retrieved the saved note. Qwen
  then performed FigCraft discovery → schema lookup → actual `inspect_document`
  invocation. The returned untitled in-memory document was accurately described
  without claiming a new exported file. The note was present in the actual tool
  result even though the final prose did not quote it.
- The combined desktop research run timed out twice fetching Example Domain,
  despite the earlier successful backend fetch. Its journal state was `failed`
  and its final answer began with the runtime's explicit incomplete-work warning.
  Network reliability is therefore **not accepted as consistently working**.
  Authenticated hosted search remains **skipped, missing credential**. No TLS,
  proxy, redirect or private-network protection was disabled to obtain a pass.
- Final synthetic MCP tests ran with a deliberately set non-secret parent
  environment marker and confirmed that marker was absent in the child. Both
  real native workflows passed again; no fixture Craft CLI processes remained.
  VectorCraft still disconnected and remains unsupported on this installation.
- Distribution: `dist/VMT Local Agent 0.4.0.exe`, **15,400,448 bytes**, SHA-256
  `C3404573EA4F8ED84E59815E845C5FFA1021DFB7487892684C517E6B54AF1C43`.
  The existing desktop shortcut now targets this version; its previous shortcut
  was backed up under `.local`. The older 0.1.0 application window was preserved.
- Publication target: the current feature branch and
  [draft PR #4](https://github.com/Mabonax/vmt-local-agent/pull/4), retaining its
  0.3.0 base. The exact published commit/ref comparison is recorded in the PR and
  final handoff because a report cannot include its own content-addressed commit
  hash. No merge or force-push is part of this work.

**Assessment:** acceptable as a verified, trusted-workspace local developer
preview with Qwen Agent mode and DeepSeek Chat/guided Edit. It is not an enterprise
or clinical production release, a generally reliable DeepSeek autonomous agent,
or a guarantee of Codex parity. Keep PR #4 in draft pending acceptance of the
documented external compatibility and security limits.

Priorities: (1) OS-enforced process/filesystem/egress isolation and plaintext
data governance before sensitive use; (2) reliable research networking, a
user-provisioned hosted-search key and VectorCraft compatibility; (3) model
workflow/completion quality, stronger action identity tracking and denied-action
deduplication; (4) journal retention/indexing, cross-process updates, richer
grounded citations, and additional native/office application coverage.

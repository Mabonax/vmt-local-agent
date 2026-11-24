# VMT Local Agent unified integration audit

Work started 24 November 2026 and continued into 25 November 2026, Africa/Johannesburg.
This report distinguishes deterministic runtime acceptance, local-model behaviour
and visual review. The application is not certified for unattended production or
for processing hostile binary documents.

## Repository preservation and integration

Repository: `Mabonax/vmt-local-agent`. Integration branch:
`codex/local-agent-unified-integration`. No push, PR, merge into `main` or release
publication was performed.

| Item | Verified state |
| --- | --- |
| Starting branch | `codex/design-skills-integration` |
| Starting HEAD | `68100c83cc97057765b5d50663669d8626dc126d` |
| Shared baseline | `5d56e91a530c9278081221c81dae68117ea411cc` |
| Remote branch | `origin/codex/tool-pack-foundation` |
| Incorporated remote HEAD | `58d5f6184baa7927ef7da1db578785a3fe46c4f3` |
| Remote scope | 105 commits beyond `origin/main`; 53 added files; 3,246 lines |
| Preservation commit | `730b4c2` preserves the four existing document-trial changes |
| Foundation merge | `cf5c6f6` incorporates the remote tree without replacing local Rust development |
| Runtime/security/acceptance | `e2c7e60` |
| Desktop controls | `e688c8e` |

The four pre-existing changes were `src/check.rs`, `src/lib.rs`,
`src/document_trial.rs` and `scripts/accept_document_authoring.py`. None was
discarded. Existing skills/design, memory, permission grants, task journals,
research, MCP, Chat, Agent and Edit implementations were retained. Local branch
refs for permissions/skills, design V2, document fixes and the earlier policy/
research work were inspected before choosing the most complete desktop base.

Recovery ref: `recovery/pre-unified-20261124-213326`. The ignored backup directory
`.local/unified-integration-20261124-213326/` contains a verified `before.bundle`
including all refs/stash refs, ref/status inventories, `working-tree.patch`,
copies of untracked/modified files and profile configuration backups. A detached
`remote-baseline` worktree retained the exact original remote version for tests.
No reset, clean, force push or stash application was used.

The remote changes were additive Python files, not a replacement Rust desktop.
Main conflicts were architectural: raw binary envelopes versus model-safe
artifact references; in-memory state versus restart persistence; Python-only
approvals versus Rust's review broker; standalone discovery versus Ollama tools;
and overlapping native design, research and knowledge facilities.

## Environment and dependency decisions

Verified workstation: Windows 11 Pro AMD64, build 10.0.26300; Rust/Cargo 1.100.0;
Python 3.14.5 default with Python 3.13 available; Node 24.21.0; Ollama 0.40.16.
Hardware: 31.73 GiB RAM, RTX 3060 Laptop with 6,144 MiB VRAM, NVIDIA driver 621.14.
At initial inspection, RAM available was 13.05 GiB and C: free space 59.52 GiB.
Later idle RAM available was 11.26 GiB; this is not peak-use telemetry.

Existing models included `qwen3.5:4b`, the local DeepSeek 7B model and local
coding models. No model was downloaded. Actual acceptance used Qwen 3.5 4B,
16,384 context tokens, 2,048 output tokens, review mode and no internet inference.
Ollama `/api/ps` reported `context_length: 16384` and 3,555,731,700 bytes of model
VRAM allocation during a trial. No out-of-memory error occurred. This does not
establish peak GPU usage or safety of a larger context or model. Normal settings
remain at the previously selected 16,384/2,048 limits.

Two isolated Python 3.13 environments were used: the original-version baseline
under the backup directory, and the integrated environment at
`%LOCALAPPDATA%\VMT\LocalAgent\tool-python-venv`. The existing design-runtime
Python 3.14 environment was preserved. LibreOffice was already installed at
`C:\Program Files\LibreOffice\program\soffice.exe`; its executable was pinned.

| Package | Integrated version | Licence metadata |
| --- | --- | --- |
| python-docx | 1.2.0 | MIT |
| openpyxl | 3.1.5 | MIT |
| python-pptx | 1.0.2 | MIT |
| pypdf | 7.1.1 | BSD-3-Clause |
| reportlab | 4.4.10 | BSD |
| pypdfium2 | 5.15.2 | BSD-3-Clause / Apache-2.0 / bundled dependency notices |
| Pillow | 13.0.0 | MIT-CMU |
| psutil | 7.0.0 | BSD-3-Clause |
| lxml | 6.1.3 | BSD-3-Clause |
| typing-extensions | 4.16.0 | PSF-2.0 |
| et-xmlfile | 2.0.0 | MIT |
| XlsxWriter | 3.3.4 | BSD-2-Clause |
| charset-normalizer | 3.5.3 | MIT |

The remote pins pypdf 6.10.2, Pillow 11.3.0 and PDFium 4.30.0 were tested first,
then deliberately replaced. PyPI version metadata reported advisories for the
old pypdf/Pillow versions; the reviewed newer versions had no advisory entries
at inspection. PDFium was advanced to the reviewed 5.15.2 build containing newer
PDFium security fixes. This is a point-in-time metadata review, not proof that
the complete runtime has no vulnerabilities. Requirements changes are explicit.
PDFium's changed page lifetime API was corrected and tested.

Official sources used: [PyPI pypdf](https://pypi.org/project/pypdf/),
[Pillow releases](https://pillow.readthedocs.io/en/stable/releasenotes/index.html),
[Pillow security](https://pillow.readthedocs.io/en/stable/handbook/security.html),
[Pillow Python support](https://pillow.readthedocs.io/en/stable/installation/python-support.html),
[PDFium releases](https://github.com/pypdfium2-team/pypdfium2/releases).
The PyPI JSON version metadata, wheel URLs, SHA-256, licences and advisory
records are retained in `integration-wheelhouse/metadata.json` in the backup.

Normal pip downloads encountered body stalls and Windows file locks. Official
PyPI wheels were downloaded with bounded retries/range requests, verified against
PyPI SHA-256, then installed offline with `--require-hashes`. The private wheel
lock and install logs are retained. `pip check` passed. Wheels/runtime packages
were not vendored into Git. The full installed environment is not cryptographically
re-attested per call; the executable pin includes the venv launcher, not every
stdlib/library DLL. Preserve bundled third-party notices when distributing.
The pre-existing optional PyMuPDF design dependency has separate AGPL/commercial
licensing considerations; this integration does not redistribute that runtime.

Rust additions: pinned `base64 0.22.1`, `image 0.25.10` with PNG/JPEG only,
`windows-sys 0.61.2` Job Object features and POSIX `libc`. Existing locked crate
versions were preserved; the lockfile changed only for these dependencies/features.
Current crate documentation was consulted for Windows handles and egui 0.37 APIs.

## Runtime and discovery architecture

The existing agent remains the orchestrator. Its native tools, skills, memory,
MCP sessions and task journal are reused. `Proposal::ToolPack` passes through the
same Rust prepare/review/execute boundary as other external capabilities.

`src/tool_packs.rs` embeds a reviewed set of Python sources and schemas, stages
them by content fingerprint in private profile storage and verifies them before
execution. The interpreter is absolute and configurable. The application does
not trust model/workspace manifests or auto-install packages. Python starts
without a shell, in isolated mode, with an environment allowlist and bounded
JSONL pipes. Errors return structured categories without private exception text.

Workers persist per workspace, with at most two retained. Configuration/collection
changes stop the old worker; a cross-process writer lease prevents concurrent
artifact-index writes. Artifacts and FTS state survive eviction/restart. The
host-only protocol provides initialization, exact approval issuance, binary
export and artifact receipt. None is exposed as a model capability.

The final inventory is **57 host-callable capabilities**: 32 execution tools,
17 pre-existing host capabilities and eight added adapters. Three workspace
operations are included in 57. See [the complete inventory](../../tool-packs/README.md).
Existing native Rust/design tools are additional; configured MCP capabilities are
dynamic and are not included in this count.

Discovery uses the reviewed allowlist, lexical/category matches, missing-module
filtering, a configurable result limit and serialized schema byte budget. Default:
six results/6,000 bytes. Office preview is excluded if LibreOffice is unconfigured.
Availability checks establish module discoverability, not full dependency health.
The desktop provides budget controls. Native Ollama gets typed selected functions;
the JSON fallback uses a strict invocation envelope, host schema validation and
at most one malformed-protocol repair. Discovery can update the selected tool set.
No semantic tool selector or token-perfect schema budget was implemented.

## Security findings and changes

| Boundary | Implementation and acceptance |
| --- | --- |
| Host authority | Model/skill text cannot issue grants or private approval tokens. Pack proposals always require review, including Routine mode; read-only blocks execution. |
| Exact approval | Rust captures config, workspace, collection and input/artifact hashes. Python tokens bind tool/arguments/parent revision, expire in 60 seconds and are consumed once. Mutation, replay and expiry tests pass. |
| Workspace access | Existing resolver runs before review and at dispatch. Paths, traversal, secrets, devices, streams and junctions retain their guards. Model file tools additionally reject the private application profile. |
| Input races | Ingestion/import use bounded reviewed byte snapshots; a changed input between actual CLI review and dispatch was rejected without ingesting the replacement. |
| Runtime integrity | Compiled allowlist/schema catalogue and staged-source SHA checks fail on drift; operator configuration pins executables. This does not eliminate malicious local-user replacement races or dependency tampering. |
| Windows processes | Root Job Object: kill-on-close and 2 GiB tree memory ceiling. Leaf jobs: 512 MiB per process, CPU limits and wall/output deadlines. Memory exhaustion, CPU exhaustion and owned-grandchild termination tests pass. |
| Cancellation | Rust polls cancellation/deadlines while writing, receiving and waiting for the manager lock; failed workers are removed. Active Office preview cancellation killed Python, conhost, soffice.exe and soffice.bin descendants, with no survivors. |
| POSIX | Worker/process groups, Python-leaf address-space/CPU/file-size rlimits and cooperative cancellation are implemented. Native Office conversion inherits the worker address-space ceiling rather than the Python-leaf CPU wrapper. No Linux execution acceptance was performed. |
| Network | Web access requires the Rust internet flag plus an exact review. Checked public DNS addresses are pinned into TLS connections; private, mixed-public/private, literal-IP, credential, redirect and unsafe destination cases are rejected by existing tests. Live example.com retrieval passed. |
| Untrusted content | Documents/webpages/MCP descriptions remain evidence. They cannot grant permissions. Prompt instructions alone do not provide comprehensive injection resistance. |
| Binary context | Host artifact references replace binary envelopes. Supported MCP binary content is registered privately when possible; otherwise omitted with a compact marker. Arbitrary encoded data hidden in ordinary text cannot be reliably recognized. |
| Secrets/logs | Worker environments omit credential variables. Host errors omit document text/URLs. Private review displays and user-selected acceptance transcripts are not secret redaction guarantees. No normal-profile credentials were printed or sent to external inference. |

The app is **not an OS filesystem/network sandbox**. Trusted Python parsers,
LibreOffice and reviewed MCP servers run with the user's account. Memory/CPU/job
controls reduce resource and descendant risks, not arbitrary file/network access.
MCP process handling retains its existing limitations; it was not silently granted
the new leaf isolation guarantees.

Important bounds: root JSONL 2 MB; leaf transport at most 1 MiB; RPC 120 seconds;
Office/PDF/visual leaves 30 seconds for cold imports; LibreOffice 90 seconds.
Artifacts: 700 KB each/100 records/20 MiB. PDF parsing: 500 KB/50 pages. Office
preview: all 1–40 supported pages at 72 DPI; raster images at most two million
pixels. Limits fail closed. Quotas may require manual backed-up retention; there
is no automatic deletion or retention UI. Orphan files after an interrupted
index commit require manual reconciliation. Private data remains plaintext.

## Artifact, design, knowledge, research and MCP integration

Artifact metadata is atomically persisted with immutable ancestry and hashes.
Reads/saves recheck content integrity; saves use a new explicit destination.
The desktop can open a verified private copy, save it, render Office/PDF previews,
view bounded images and request revision comparisons. Comparisons provide bounded
native text and byte differences, not a visual equivalence certificate.

Existing document/colour skills and native design adapters remain installed by
reference. Their generated supported files can register into the artifact store.
New simple Office producers use semantic Word headings, editable slide text and
literal workbook cells. The existing richer design composer remains available;
this merge did not replace it with the simpler generators. Office preview reuses
the reviewed Office safety validator, converts with a fresh LibreOffice profile,
renders every supported page through PDFium and reports page/text-bound checks.
Active objects, external relationships, dangerous fields and imported XLSX
formulas are rejected. Structural PASS always leaves visual review pending.

FTS5 collections persist separately from conversation memory and the pre-existing
optional Ollama semantic index. Ingestion is individually reviewed and supports
UTF-8 text, DOCX body/table paragraphs and PDF embedded text without OCR. Source
file and extracted-text SHA-256 are separate; immutable revisions, BM25 passage
retrieval, exact citation offsets and collection statistics were exercised across
fresh checker/worker processes. Semantic embeddings were inspected as an existing
optional enhancement; no embedding model was installed or invoked.

Reviewed HTTPS responses register source/response/text digests, timestamps and
exact passage citations. Evidence remains worker-local and does not survive
worker restart. The original native `web_search` integration remains available
only when explicitly configured/reviewed; no provider key was provisioned or live
search tested. The Python research adapter adds retrieval, not a search provider.

MCP registry/discovery/schema/approval/execution remain intact. Supported binary
results can join the artifact lifecycle. Generic MCP functions remain available
in document conversations. There is no new arbitrary MCP workflow executor:
the existing staged agent workflows can sequence reviewed MCP calls, while the
new deterministic PDF template only uses its fixed registered steps. Normal
profile MCP registry and the expected Storytold Craft CLI directory were absent;
ArtCraft/Storytold live acceptance is therefore **unverified**. No server was
installed or launched to manufacture a pass.

## Baseline and final automated verification

Windows Rust commands use `build.ps1` to initialize VS 2022. Final commands:

```powershell
cargo fmt --all -- --check
./build.ps1 -Action check -CargoArguments @('--workspace')
./build.ps1 -Action clippy -CargoArguments @('--workspace','--all-targets','--','-D','warnings')
./build.ps1 -Action test -CargoArguments @('--workspace')
./build.ps1 -Action build -Release
& $toolPython -B tool-packs/check_packs.py
& $toolPython -B -m unittest discover -s tool-packs -p 'test_*.py' -v
& $toolPython -m pip check
```

Here `$toolPython` is the configured Python 3.13 venv executable.

| Suite | Original baseline | Integrated final |
| --- | ---: | ---: |
| Rust library | 48 pass | 55 pass |
| Rust integration | 3 pass | 3 pass |
| Python execution-pack unit tests | 35 pass | 35 pass |
| Python host/runtime unit tests | 37 pass | 48 pass |
| Full pack acceptance wrapper | 1 pass | 1 pass |
| Total unittest/test executions | 124 pass | **142 pass, 0 fail** |

The acceptance wrapper separately exercises all 32 registered leaf tools with
valid and invalid example calls. It is counted once above; the examples are not
inflated into additional unittest counts. fmt/check/Clippy/release build pass.
New tests cover durable ancestry/corruption, schema/catalog parity, token misuse,
source revisions, formula rejection, writer leases, workflow failure propagation,
Windows resource/descendant limits, deadline/cancellation and missing-output claims.
CI configuration now runs all Python host tests on Windows/Linux with Python
3.12/3.13, but hosted CI was not run because this branch was not pushed.

## Actual acceptance scenarios

The repeatable operator-owned fixture approves only explicit synthetic inputs;
it is not a grant supplied by the model. Commands actually executed:

```powershell
& $toolPython -B scripts/verify_unified_integration.py --checker target/release/vmt-agent-check.exe --root .local/unified-integration-20261124-213326/acceptance-3 --office 'C:/Program Files/LibreOffice/program/soffice.exe' --live-https
& $toolPython -B scripts/verify_unified_security.py --checker target/debug/vmt-agent-check.exe --fixture .local/unified-integration-20261124-213326/acceptance-3
```

The first command requires a NEW fixture directory on repetition. The second
requires that fixture's desktop profile to be closed.

| Scenario | Observed result |
| --- | --- |
| A deterministic Word | PASS: native editable DOCX, three headings and expected budget, persisted registration, second revision/ancestry and content comparison. Opened a verified copy in Microsoft Word through the desktop. Model content acceptance is separate below. |
| B PowerPoint | PASS: four editable widescreen slides; reopened with python-pptx; all four pages rendered and visually inspected. |
| C Excel | PASS: literal financial rows, budget 375,000/actual 365,000/variance 10,000, reconciled totals, styled headers/freeze panes; reopened and rendered. No formulas claimed. |
| D PDF workflow | PASS: fixed generate/inspect/render/image/hash steps and gates; text reopened, fingerprint verified and preview inspected. |
| E knowledge | PASS: authorised synthetic file ingestion, exact source fingerprint/citation, statistics and retrieval after fresh host/checker starts. Model answered the three facts with real passage offsets. |
| F research | PASS: exact reviewed HTTPS fetch of example.com, source registration, passage retrieval and fingerprints; unapproved/private requests rejected. First run had a DNS error; subsequent repeated runs passed. |
| G security | PASS: invalid fields, traversal, raw envelopes and disabled/private networking rejected. Separate real CLI review rejected concurrent input replacement. Active Office cancellation completed in 1.55 seconds with five observed owned descendants and no survivors. Token replay/expiry/argument mutation/output bounds/resource abuse are separately covered in automated tests. |
| Office preview | PASS: Word one page, PowerPoint four pages, Excel one page; all pages rendered with no out-of-page text findings. Seven deterministic page images and one generated-model Word page were visually inspected. |

Final deterministic acceptance: **8 pass/0 fail**, plus **2 actual review/
cancellation checks pass**. Earlier failures are retained in `acceptance-1`:
PDFium page context-manager incompatibility, cold 10-second import timeouts and
two acceptance result-shape errors. They were corrected rather than disabled.
`acceptance-2` and final `acceptance-3` passed all eight cases. Failed workflow
status now propagates as failed host execution rather than a successful envelope.

## Real local model results and context

Scenario H used the installed Qwen 3.5 4B, with no downloaded models or remote
provider. The explicit `--unified-live` harness runs the real agent, tool review
broker, skills and Ollama in an isolated `.local` profile. It allows only named
synthetic capabilities and rejects arbitrary writes, commands, network and MCP.
Its task budget is three minutes/18 actions and uses the bounded host inference
deadline. Artifacts before/after, actual events and final/errors are recorded.

| Trial | Observation | Assessment |
| --- | --- | --- |
| Native Word, 21.85 s | Loaded document-design V2 skill, discovered tools, produced an actual four-section DOCX with an opaque ID and no tool errors/denials. | Tool/skill/registration PASS; factuality FAIL: invented successful test results/zero incidents. |
| Native Word retest | Added a general factuality instruction; model loaded the document skill and generated the required four-section file, expected budget/duration/owner, with two sentences per section. Rendered and visually inspected. | Structure/layout PASS, content PARTIAL: still claimed mitigation strategies had already been developed without evidence. Not publication-ready. |
| Native PDF workflow, 33.93 s | Called `pack__workflow_run`; all five gates passed; returned actual PDF and PNG IDs/hashes. | PASS for this bounded request; visual review remains separate. |
| Native knowledge, 7.55 s | Called indexed search; answered budget, twelve weeks and Technical Specialist; cited the actual document ID and offsets 0–104. | PASS; no web request or fabricated citation. |
| JSON fallback initial | Repeated `read_file src/main.rs` in a document-only fixture; no artifact generated. | FAIL, safely paused after repeated errors. |
| JSON fallback retest, 12.31 s | Removed coding-specific example priming and scoped JSON instructions; model returned completion without tools despite correction. | FAIL, runtime rejected the completion claim. No action was executed. |

The final scenario H result is **PARTIAL**. Native tool use works on the tested
small workflows; generic JSON fallback and factual autonomous report writing
are not reliable. Structured parsing, exact schemas and missing-output checks
prevent malformed actions/empty completion claims; they cannot verify the truth
of arbitrary generated prose. The original complex document add/edit trials are
preserved and remain failures (missing tables, incomplete append/edit outputs and
an incorrect numeric edit). This integration does not turn those into passes.

Discovery limits, message compaction including native call JSON, bounded output
and 16K context were exercised. Actual prompt/evaluation token counts were not
persisted per turn; `/api/ps` proves the loaded context allocation, not exact
context consumption. A final native knowledge trial against the final release
also passed in 10.30 seconds with no errors or denials after the explicit
cloud-model metadata rejection guard was compiled. No claim is made that increasing context further would
improve small-model planning or be resource-neutral.

## Desktop startup and visual acceptance

The optimized desktop and checker were built with VS 2022. The desktop launched
in an isolated acceptance profile, retained Chat/Agent/Edit controls and showed
Local connectivity. Settings → Tools & artifacts loaded **57/57 dependency-
discoverable capabilities**, artifact hashes/ancestry and working open/save/
preview/review controls. An actual Open verified copy action opened the generated
report in Microsoft Word; its text and page were inspected without changing the
user's pre-existing CV document.

Windows accessibility exposed the working controls and updated results, but
Windows Graphics Capture returned a white client image for the OpenGL agent
window, including the previous build. Some off-screen input actions also lacked
geometry. Consequently a complete visual desktop QA pass is **not claimed**.
Word screenshots and rendered document PNGs were readable. Backend preview/save/
comparison paths were exercised through the actual Rust broker; not every
desktop button or native save dialog was exercised through UI automation.

The final release was subsequently launched with the normal profile: existing
`john work` project/chats remained available, Qwen 3.5 4B was selected, and status
was Local/Ready. Its runtime inventory reported 57 registered capabilities, no
missing modules and LibreOffice available. The prior internet setting was true
both in the initial backup and after launch; it was preserved, not enabled by
this integration. Every fetch still requires exact review. The 16,384/2,048
context/output settings were likewise preserved.

The local `dist/VMT Local Agent 0.6.2.exe` used by the existing Desktop shortcut
was refreshed from the tested optimized build; the old executable was backed up
as `desktop-062-before.exe` in the recovery directory. Both executable copies
have SHA-256 `b803aa134e326fc7435779c932523dae61f2a4017f4e6f6bc905b9e4c79a04b5`.
This was a local copy/update, not a GitHub release publication. The app is left
running on the normal profile.

## File groups changed and remaining work

- Rust: `tool_packs.rs`, `unified_trial.rs`, agent/Ollama orchestration, types,
  policy, execution, tools, MCP binary handling, workspace/private-path guard,
  task permission/evidence logic and checker CLI.
- Desktop: `tool_ui.rs`, `app.rs`, `main.rs`; existing navigation and design kept.
- Python: durable artifact store, worker/host adapter, process/leaf limits,
  cancellation propagation, discovery/catalog, source-fingerprinted knowledge,
  reviewed dependencies/manifests and integration tests.
- Acceptance/operations: two repeatable verification scripts, Python CI coverage,
  main/runtime READMEs and this audit. Generated outputs/logs stay ignored.

Next development should prioritise factual source-constrained generation and
tested small-model fallback, richer document content-preservation gates, real
Linux acceptance, OS filesystem/network sandboxing and full dependency
attestation. Also outstanding: normal-profile artifact retention/encryption,
research persistence, OCR, advanced workbook formulas/layouts, stable OpenGL
capture/desktop visual QA and actual approved Craft MCP acceptance. General
workflow composition and cross-application rollback are not implemented.

## Launch and reproduce

From `C:\xampp\htdocs\vmt-local-agent` with Ollama running:

```powershell
./target/release/vmt-local-agent.exe
```

To rebuild first:

```powershell
./build.ps1 -Action build -Release
./target/release/vmt-local-agent.exe
```

For an isolated profile/workspace:

```powershell
./target/release/vmt-local-agent.exe --data-dir C:/path/to/private-profile --workspace C:/path/to/project
```

On another installation, configure its absolute tool Python/LibreOffice paths
using the main README before refreshing capabilities. Do not share a running
desktop's workspace/profile with CLI artifact writers. Use `git status --short`,
`git log -5 --oneline` and `git rev-parse HEAD` for delivery state; this report is
committed after the implementation commits listed above.

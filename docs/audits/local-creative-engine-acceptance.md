# Local creative engine acceptance

Date: 24 November 2026, Africa/Johannesburg. Repository: `Mabonax/vmt-local-agent`.

## 1. Executive summary

The remote creative engine and the separate local desktop/document work have
been integrated on `codex/design-skills-integration`. The application builds and
launches on this Windows workstation. Final local validation passed **50 Rust
tests, 43 Python tests and all five creative benchmark assignments**. The actual
broker generated editable Office files, SVG/PNG/PDF graphics and processed images.
Every page and slide in the final primary artifact set was visually inspected.

Acceptance is qualified: deterministic execution is operational, while broad
multi-action local-model tasks remain unreliable. Real native and JSON inference
failures are retained, not relabeled as successful tasks. Vision works with strict
validation and a disclosed, bounded coarse-region compatibility retry. This is an
operator-mediated creative workflow, not a fully autonomous creative director.

The operator requested a context increase during acceptance. The saved normal
profile now uses **16,384 context tokens**, previously 8,192, with its existing
2,048 output-token setting and permissions preserved. Measured 16K execution fit
the GPU. More context can increase latency and memory pressure; 32K was not tested
or enabled.

## 2. Repository synchronisation

Initial branch: `codex/design-skills-v2`, commit
`efb6fa33c196d35cf0e8c79f86fcc6f3163398b1`. There was one modified setup script
and two untracked V2 scripts. They were preserved in:

- Recovery branch `recovery/pre-creative-acceptance-20261124`.
- Stash `Recovery: local design-v2 work before creative acceptance 2026-11-24`.
- Ignored `.local/acceptance-20261124/pre-sync.patch` and copies of the two scripts.

`git fetch origin --prune` advanced the remote integration branch from `9ded12a`
to the supplied `6c5761d4d47f3753eeeb059c238dcbf4db01d1a4`. Twenty newer remote
commits were fast-forwarded into the local integration branch. The one divergent
local commit was merged, preserving its document formatting, completion checking,
clipboard and desktop work. Three overlapping files required reconciliation:
`src/design_tools.rs`, `src/ollama.rs` and `scripts/design_runner.py`.

Merge commit `c046a87` has both `6c5761d` and `efb6fa3` as parents. The unfinished
uncommitted V2 experiment remains in recovery storage rather than overwriting the
remote V2 implementation. No destructive reset, force push or merge into main was
used. Final delivery SHAs are verified separately after committing this report.

## 3. Hardware profile

| Item | Observed value |
| --- | --- |
| OS | Windows 11 Pro, AMD64, build 26300 |
| CPU | Intel Core i7-12700H, 14 cores / 20 logical processors |
| RAM | 33,272,312 KiB visible, approximately 31.73 GiB |
| Initial free RAM | Approximately 7.5 GiB |
| RAM at context increase | 6,255,464 KiB free, approximately 5.97 GiB |
| GPU | NVIDIA GeForce RTX 3060 Laptop GPU |
| VRAM | 6,144 MiB |
| Driver | 621.14 |
| GPU usage at context increase | 4,985 MiB used |
| Initial free disk | C: 64,649,879,552 bytes; J: 172,318,662,656 bytes |

Ignored `metrics.json` contains sampled system free RAM and GPU usage during
builds and inference. These measurements include other open applications and
compilation; they are not isolated model-only RAM measurements. The 600-sample run reached a maximum of 5,275 MiB GPU memory, below the 6,144 MiB capacity. Concurrent builds
and inference temporarily left only about 0.48 GiB free RAM. Avoid treating that
minimum as the normal idle requirement, and avoid large concurrent model loads.

## 4. Environment and runtime

| Component | Verified installation |
| --- | --- |
| Rust / Cargo | 1.100.0 / 1.100.0 |
| Python | 3.14.5 |
| Design Python | `%LOCALAPPDATA%\VMT\LocalAgent\design-runtime\venv\Scripts\python.exe` |
| LibreOffice | 26.2.2.2, `C:\Program Files\LibreOffice\program\soffice.exe` |
| Ollama | 0.40.16, `http://127.0.0.1:11434` |
| Fonts | Arial verified; Windows Fonts inventory inspected; worker uses its font allowlist |
| Desktop version | 0.6.2 |

The existing private environment exactly matched all seven pinned dependencies:
Pillow 12.3.0, python-docx 1.2.0, python-pptx 1.0.2, PyMuPDF 1.26.7, lxml 6.1.3,
typing_extensions 4.16.0 and XlsxWriter 3.3.4. `pip check` passed. No system-wide
Python installation or model download was performed. Setup `--help` was checked;
existing valid executable pins and environment were preserved. Actual headless
Office and SVG conversion verified the renderer, not merely its executable path.

Private settings, registry and runtime snapshots are under the ignored acceptance
directory. The context change also has its own pre-change settings backup there.

## 5. Installed skills

All **30 portable skills** are enabled: 18 existing VMT guidance skills and these
12 creative packages. Existing guidance sources were preserved.

| Package | Source / verification |
| --- | --- |
| colour-intelligence | Existing reviewed private package; original ZIP hash matched |
| professional-document-design | Existing reviewed private package; registry fingerprint matched documented value |
| colour-intelligence-v2 | Existing reviewed private package, fingerprint `0448616d3162505e11b7c53b0114ebebb41afa5146f0d8ecab8dccecfdff2291` |
| professional-document-design-v2 | Existing reviewed private package, fingerprint `621572f0ab49b91734fbf88f59cbaa19c4f8b053ff1e1d441771af025322ab46` |
| typography-intelligence | Reviewed repository package |
| layout-intelligence | Reviewed repository package |
| vector-graphics | Reviewed repository package |
| image-intelligence | Reviewed repository package |
| creative-templates | Reviewed repository package |
| creative-director | Reviewed repository package |
| vision-intelligence | Reviewed updated repository package, including bounded retry disclosure |
| scene-repair | Reviewed repository package |

The located archive was `Downloads\colour-intelligence (1).zip`, SHA-256
`7620aba647a80f2085fe41f859aadb2061c5e816f071cde9989ae24c8f6238ee`.
Searches of Downloads, Desktop, Documents, Codex attachments and existing project
fixtures did **not** locate the other three original archives under their expected
or related names. Their installed packages already existed and passed registry
drift checks. No missing archive was fabricated. Therefore the V2 archive hashes
listed below were not freshly revalidated against ZIP files in this run:

- Colour V2: `20d4fad76e54bfb54643fb08c2273fb9b0ba544eb9da757a927f436ebc4ff381`.
- Document V2: `a83e33b35a0ba7d30b6099fb13078f141d58e31a9439c17cfe9ec4dbe8744e28`.

The registry verified metadata discovery, progressive loading, inventoried resource
access, approval pins, disabled packages and drift rejection. A regression test
explicitly verifies `SKILL.md` as a reviewed fixed-worker package resource without
executing the manifest. Registration never grants runtime permissions.

## 6. Compilation

Formatting, locked compilation, all-target strict Clippy and locked release build
passed using `build.ps1` to select the complete VS 2022 x64 toolchain. Both
`target\release\vmt-local-agent.exe` and `target\release\vmt-agent-check.exe`
exist and were launched. The release rebuild after corrections took 77 seconds;
the final unchanged build confirmation took 0.84 seconds.

Windows initially rejected every real broker dispatch because the embedded worker
exceeded the command-line limit when supplied using `python -c`. Staging exact
compiled-in worker bytes in the private invocation directory corrected this.
Compilation alone had not exposed that defect.

## 7. Rust tests

Final result: **47 library tests and three integration tests passed**, zero failed
or ignored. The three integration binaries cover design actions, desktop workflows
and permissions/portable skills. Library execution took approximately 2.2 seconds;
integration bodies took approximately 0.2, 0.4 and 2.0 seconds respectively.

Added coverage includes executable drift, disabling a package, manifest-backed
adapter resources, and space for ordinary instructions/tool results in an 8K/4K
JSON fallback context. Existing grant quotas, review mismatch, input/asset drift,
read-only, cancellation, workflow and session/memory checks remain enabled.

An additional explicit temporary-project runtime acceptance passed policy review,
denial, concurrent edit protection, command failure/recovery, skill loading,
memory isolation, workflow transitions and uncertain journal handling.

## 8. Python tests

Final result: **43 passed**, zero failures. Final execution took approximately
2.1 seconds in the configured isolated interpreter.

Tests cover all worker families, V2 validation and bounded repair, native Office
structures, external/active Office content rejection, typography, layout, shape
serialization, asset decoding, all image adjustments/masks, structured vision,
scene repair and stale/mismatched provenance. New pixel assertions reproduce
embedded-image export, fit/contain padding, cover and effective opacity in PNG/PDF.
Animated extraction and oversized decoded images are rejected. Gradient export
fails closed without the reviewed alternative renderer. A genuine python-pptx
save/reload test verifies removal of empty generated continuations only.

Vision retry tests preserve rejection of malformed responses, enforce a fresh
second response and whole-image region, and reject a second invalid review.

## 9. Creative benchmark

| Assignment | Final result |
| --- | --- |
| Poster | PASS |
| Social graphic | PASS |
| Business card | PASS |
| Certificate | PASS |
| Contrast repair | PASS |

The original xlink namespace correction passed the initial local benchmark. Its
embedded red-region pixel assertion was preserved. Subsequent tests discovered
additional MuPDF image-fit, image-opacity and gradient defects that the original
red-region assertion did not detect. Those now have separate regression/evidence
coverage. All five assignments passed again after the corrections. Artifact
hashes, scene/native SVG text, output dimensions and pixel evidence were checked.

Final benchmark evidence:
`.local/acceptance-20261124/creative-benchmark-delivery/report.json`.
These worker fixtures do not certify aesthetics or local-model task completion.

## 10. Artifact inventory

Primary workspace, outside production sources:
`C:\Users\John Mabona\AppData\Local\VMT\CreativeAcceptance\20261124-final`.

| Editable source | Rendered pages/slides |
| --- | ---: |
| professional-cv.docx | 1 |
| business-proposal.docx | 1 |
| multi-page-report.docx | 2 |
| native-table-report.docx | 1 |
| long-content-report.docx | 3 |
| formatted-cv.docx | 1 |
| baseline.docx | 1 |
| business-presentation.pptx | 3 |
| ten-slide-presentation.pptx | 10 |
| native-table-presentation.pptx | 2 |
| long-content-presentation.pptx | 6 |

The primary set contains **11 Office files / 31 rendered pages and slides**,
including the supplemental baseline render. Native text and tables were inspected
with python-docx/python-pptx, and source wording was checked across continuation
boundaries. The V2 native-table presentation contains a native editable table.

Graphics include event poster, business card, social graphic, certificate and a
multi-image gradient/wrapped-text scene, each retained as JSON, SVG, PNG and PDF.
Feedback originals, linked evidence, repair proposal and separately rendered
candidate are preserved. Twelve representative image assets/outputs include the
original synthetic landscape, contain resize, cover crop, four enhancements,
grayscale, transparent/adjusted logo, circular portrait and rounded image.

Inputs are synthetic illustrations, not real personal photographs. The tests
establish processing semantics; photographic enhancement quality was not certified.
Native local-model DOCX/PPTX outputs from two live workspaces were separately
rendered, four additional Office files / eight pages or slides. Their receipts are
in the parent `supplemental-artifacts.json`.
The primary `artifact-inventory.json` records paths, byte sizes and SHA-256 values
for 148 retained files at inventory time, including separately labeled negative
fixtures and review images. Later evidence files may increase that count.

## 11. Actual visual inspection

All 30 primary review pages/slides preceding the supplemental baseline were opened
in labeled contact sheets. The baseline and live outputs were rendered separately
for additional inspection. PNG and PDF versions of each graphic were inspected;
SVG XML, text preservation, embedded data URLs, namespace and corresponding
renders were checked. Native browser opening of a local SVG was blocked by the
browser URL policy; no workaround or external upload was attempted.

The final primary pages show consistent fonts, readable hierarchy, clear margins,
preserved tables and no observed clipping, overlap or unexpected empty page.
The empty business-presentation continuation and Word title rule seen in the first
review were corrected and re-rendered. Long slides preserve text but may split a
paragraph across continuation slides. This is usable but remains an editorial
quality limitation. The template fixtures are restrained and sometimes sparse;
they are not a universal professional-design certification.

Graphics retain their images, correct aspect/fitting and alpha. The gradient
no longer becomes black. LibreOffice PDF gradients can show light banding at
certain preview scales; print/ICC fidelity and all SVG features remain unverified.
Intended cover crops are distinguished from accidental clipping.

The feedback poster visibly changes a barely readable light-gray title to black.
Source wording and asset references remain identical. Model findings are advisory
and are not substituted for this inspection.

## 12. Live local-model execution

Only installed local `qwen3.5:4b` was used for agent/vision inference. No mock model
was used for these runs. Metadata advertised tools, thinking, completion and vision.

| Run | Observed outcome |
| --- | --- |
| Native creative, 8K context | 26.77 s; 28 completed read/load actions, zero design approvals; step budget pause |
| JSON creative, 8K | 6.43 s; context-budget failure after inspection; no design dispatch |
| Native creative, 16K | 69.34 s; six exact approvals and six completed creative operations; all four expected artifacts existed; final model response incomplete |
| JSON creative, 16K | 16.05 s; two invalid nested `tool` argument attempts rejected; repeated-action pause; zero approved creative actions |
| JSON basic file read before prompt correction | Read succeeded, then context-budget error |
| JSON basic file read after prompt correction | PASS, 3.1 s; exact hello.txt result; no changes or commands |
| Desktop contrast | Skill loaded, exact review displayed, contrast completed at 21:1; two successful actions, no failed/denied actions |

Further fresh native/JSON creative runs after reducing prompt duplication remained
incomplete. Native completed five approved actions but requested two unlisted
render actions, which the fixed fixture denied. JSON attempted an unavailable
file/read and did not complete the creative sequence. Those are retained as
failures; the fixture allowlist was not relaxed to manufacture a passing score.
Those full model scenarios preceded the final catalogue-description shortening;
catalogue completeness was rechecked after that metadata correction.

The global prompt now defers detailed design contracts to loaded skills and
clarifies top-level tool arguments, concise summaries and advisory limitations.
This fixed the small JSON regression, without changing parser rejection or grants.
It does not establish reliable completion of arbitrary long creative tasks.
The final primary broker recipe retained 81 operation attempts: 79 successes and
two rejected structured design reviews. Subsequent compatibility/feedback receipts
record successful corrected vision execution without rewriting those failures.

Raw synthetic run logs are in the ignored acceptance directory; live reports and
artifacts are in `20261124-run3` and `20261124-live-stable` beside the final workspace.

Installed model audit: DeepSeek R1 local 7B (3.81 GB), five Qwen35-derived names
including native qwen3.5:4b (2.71–3.32 GB), llama3.2:3b (2.02 GB), embeddinggemma
(0.62 GB, embedding only), gemma4 (9.61 GB, vision/audio/tools advertised), and a
cloud-tagged MiniMax entry. Metadata capabilities are declarations, not acceptance
of every model. Only qwen3.5:4b has current live creative evidence. Gemma4 exceeds
VRAM capacity for full GPU residency and was not loaded during this run. The cloud
entry was neither invoked nor downloaded. Actual qwen 16K residency was reported
by Ollama as approximately 3.6 GB and 100% GPU; advertised maximum context is not
the measured affordable operating context.

## 13. Vision and feedback loop

All five modes were exercised through the actual approved broker against graphics,
a rendered Word page and a rendered slide: describe, extract, design_review,
document_review and slide_review. Four modes succeeded in the initial final run;
design reviews sometimes returned out-of-image rectangles and were rejected.
For example `[0,0.25,1,1]` is invalid for x/y/width/height despite being valid JSON.

The installed model supports schema-constrained responses but not consistently
their semantic geometry constraints. Disabling thinking preserves the observation
budget; context is explicitly bounded to 8K for the vision worker. Incomplete
length-limited responses are rejected. A single fresh compatibility attempt uses
a constrained whole-image region. This deliberately reduces localization accuracy,
does not repair malformed output, and is disclosed in `review_attempts` and
`region_scope`.

Actual compatibility acceptance passed on the poster in **34.04 s** and the first
feedback graphic in **17.51 s**. Both recorded first-response rejection followed
by a valid fresh whole-image response. `/api/ps` was empty immediately afterward,
so unloading was observed in those tests, not inferred solely from keep_alive=0.
Reasoning/vision lifecycle coordination remains unimplemented.

The complete operator-mediated feedback demonstration retained:

1. Original brief and Creative Director candidates in the primary receipt.
2. Selected poster scene and a controlled low-contrast source variant.
3. Deterministic low-contrast finding and original PNG render.
4. Real local vision review and actual render/image hashes.
5. Linked review accepted as `CONSISTENT_UNAUTHENTICATED`.
6. Object matching and proposal changing title fill `#EEEEEE` to `#000000` only.
7. Explicit fixture review of that exact proposal, then new JSON/SVG/PNG/PDF outputs.
8. Visual before/after inspection, with original wording/assets retained.

Original scene hash:
`660e391c520e35e5bbe1ba69bec29aecbe276203b06ed8d1cfa446746f5c58eb`.
Candidate scene hash:
`27b2968a19a6648dbff4f66ae47fc6433354cf254a758d5b93d9083dca8caa24`.
Evidence: `feedback-loop.json`, `feedback-proposal.json`,
`feedback-linked-request.json`, `feedback-poster-original.png`,
`feedback-candidate.json` and `feedback-candidate.png` in the final workspace.

The operator fixture approval is part of a controlled test; production human
acceptance remains separate. Coarse findings can match several objects. Only
independent deterministic evidence authorizes the proposed change. Contrast
criticism is conservative and is not reliable for every layered/gradient/photo
background. Unsupported visual suggestions never become executable instructions.

## 14. Security regression

The final negative-request run rejected all **16** cases and preserved original
files. An earlier 17-case run additionally verified a real Windows junction pointing
outside the selected workspace. Cases cover traversal, absolute/protected paths,
overwrites, extensions, unexpected fields, executable requests, raw SVG, external
SVG URLs, unapproved images, animated images/extraction, decoded pixel limits,
cloud models and invalid vision modes.

Two additional reviewed synthetic worker fixtures independently demonstrated:

- Oversized response rejection: `Design result exceeds limit`, 3.49 seconds.
- Worker timeout: `Design operation timed out`, **120.96 seconds**, with owned
  worker cleanup. The fixture's code only returned oversized data or slept; it
  was isolated from the normal skill registry and never installed as a real skill.

Three additional real broker calls rejected a stale scene link, a mismatched image
fingerprint and an installed non-vision model before inference. Evidence is in
`additional-rejections.json` in the primary workspace.

Rust tests cover changed assets/inputs/packages/executables after review, disabled
skills, readonly, proposal/tool mismatch, grants not covering design execution,
and cancellation. Python tests cover hostile Office relationships/entities,
malformed findings, tool-bearing vision output, stale scenes and mismatched image
links. Loopback/no-proxy/no-redirect transport, response caps and model-installation
checks remain intact. No inference endpoint other than loopback was used.

The desktop Stop action was observed reaching `Stopped`, with the explicit message
that completed actions remain and no further actions will run. This does not prove
server-side inference was immediately cancelled.

Provenance records establish consistency, **not authenticity**. They are not a
signed persistent ledger. The worker, native libraries and LibreOffice execute
with the Windows user's privileges, **not an OS sandbox**. Dependency attestation,
host compromise and existing filesystem race limitations remain outside this
application boundary. Diagnostic worker errors remain redacted; synthetic-only
diagnostic payloads are ignored local evidence, not committed logs.

## 15. Capability maturity inventory

L0 documented, L1 discoverable, L2 executable, L3 automated tests passed, L4 real
local-model verified, L5 artifact plus visual acceptance verified. L5 below is
limited to the tested operator-mediated fixtures, not arbitrary autonomy.

| Skill | Operations and highest supported evidence level |
| --- | --- |
| Colour V1 | palette L4; contrast L4; extract L3 |
| Colour V2 | repair L3; gradient L3; categorical L3; simulate L3; system L3; distance L3 |
| Document V1 | compose L3; render L5; inspect_document L3; format_document L3 |
| Document V2 | compose_v2 L5 |
| Typography | type_scale L3; measure_text L3 |
| Layout/composition | layout_grid L3; layout_check L3; layout_arrange L3 |
| Vector graphics | svg_compose L5 for tested SVG/PNG/PDF, images, gradients and wrapped text |
| Image intelligence | image_inspect L3; image_prepare L3; image_adjust L3 |
| Creative templates/critic | template_plan L3; scene_check L3 |
| Creative Director | creative_plan L3 |
| Vision intelligence | vision_inspect L4, all five modes; advisory and coarse retry limitations apply |
| Scene repair | scene_repair_plan L4; linked operator-mediated before/after artifact inspected |

All 12 creative packages are discoverable/active and all **28 registered
operations** have real broker execution evidence. L3 entries also have manual
artifacts/receipts where applicable; they are not promoted to L4 merely because a
human called the CLI. V2 composition and vector output also have native model
dispatch and separately inspected artifacts. No package is classified L5 merely
because its source exists.

The 18 other portable guidance packages are L1 for this audit: vmt-api-contracts,
vmt-audit-trails, vmt-database-migrations, vmt-feature-delivery, vmt-flutter-mobile,
vmt-flutter-ui, vmt-laravel-architecture, vmt-multi-tenancy,
vmt-production-readiness, vmt-rbac, vmt-react-inertia, vmt-react-ui, vmt-repo-audit,
vmt-ui-design-director, vmt-ui-design-system, vmt-user-journey-design,
vmt-visual-qa and vmt-workos-laravel. Their metadata and pins were checked, but
their domain workflows were not independently accepted here.

Eleven built-in guidance IDs are also discoverable (L1): code-review,
rust-development, project-discovery, word-documents, pdf-documents, spreadsheets,
presentations, artcraft-suite, test-driven-bugfix, feature-delivery and
semantic-retrieval. Guidance text does not establish that optional external MCP
providers work. This audit does not certify those unrelated application suites.

The original 5,000-character agent catalogue cut off the final creative entries:
its assembled contents were 5,837 bytes and omitted vision-intelligence from
the visible prefix. Shortening metadata descriptions to 45 characters reduced it
to 4,337 bytes, retaining all current IDs while full instructions remain
available through load_skill. The readonly `--skills-catalog` check exposes the
exact agent-visible catalogue for verification.

## 16. Defects identified

The main defects were Windows worker launch failure, provenance insertion panics
for arrays/scalars, animated extraction bypass, MuPDF gradient/image-fit/opacity
differences, empty generated continuation slides, inherited Word title borders,
global-prompt growth breaking small JSON tasks, and semantically invalid vision
regions, plus catalogue clipping that hid late creative IDs. Early local checks exposed a fixture-only Clippy warning and a Windows
executable lock during concurrent rebuild/acceptance. Both were corrected without
removing tests or weakening validation. The worker-security fixture initially used
the wrong upstream function name; that setup mistake was fixed before measuring
the actual response-cap and timeout behavior.

## 17. Corrections delivered

The broker stages compiled-in worker bytes, envelopes non-object values under
`result`, records all creative file mutations correctly, and preserves review pins.
The worker shares image decoding checks, bakes fit/alpha into bounded images,
selects the pinned renderer for gradients, removes only empty continuations,
normalizes Word title styling, and bounds strict vision compatibility attempts.
Prompt guidance uses progressive disclosure. Tests and reusable broker acceptance
scripts accompany the changes. No new dependency or automatic grant was added.

## 18. Remaining limitations

- Multi-action agent reliability is still limited; native task endings, JSON
  arguments, extra action requests and budget behavior require operator oversight.
- Vision may be inaccurate even when valid. Whole-image retry intentionally loses
  localization precision and does not establish object identity or visual quality.
- Three original external ZIPs were not located; existing reviewed installations
  remain usable, but fresh archive verification awaits those files.
- Windows worker font support remains allowlisted; arbitrary font selection,
  multilingual shaping and full accessibility certification were not established.
- Long presentations can split paragraphs. Photo/gradient/layer contrast and print
  quality need further design-engine work. Native browser SVG visual review was
  blocked; source and local renderer inspection were retained instead.
- Desktop skill discovery, project loading, model selection, review, progress,
  completed output, history and Stop were observed. Python/LibreOffice runtime
  configuration is currently CLI-based; the GUI shows status rather than a full
  runtime editor. Not every creative operation was individually invoked through
  a desktop chat; the same broker was exercised through its explicit CLI.
- Memory/session compatibility has test and existing-profile startup evidence;
  private user memory bodies were not inspected or rewritten.
- Hosted CI after publication and every alternative installed model remain
  unverified unless separately recorded. No production deployment is claimed.

## 19. Recommended next development phase

Prioritize task-sized progressive tool contracts, measured model evals, concise
completion behavior and reasoning/vision lifecycle coordination. Add deterministic
background-aware contrast and paragraph-aware slide pagination before more
autonomous repair. Then introduce before/after comparison and persistent workflow
state with explicit review/recovery. A persistent provenance ledger needs a clear
authenticity/threat model; consistency hashes alone are insufficient. Hardware
routing, Brand Intelligence, preference memory and Skill Creator Studio remain
future work, not silently implemented in this stabilization task.

## 20. Exact launch and verification commands

Run from `C:\xampp\htdocs\vmt-local-agent` in PowerShell:

```powershell
./build.ps1 -Action build -Release
./target/release/vmt-local-agent.exe --workspace 'C:\Users\John Mabona\AppData\Local\VMT\CreativeAcceptance\20261124-final'
./target/release/vmt-agent-check.exe --skills-audit
./target/release/vmt-agent-check.exe --skills-catalog
$py = "$env:LOCALAPPDATA\VMT\LocalAgent\design-runtime\venv\Scripts\python.exe"
./target/release/vmt-agent-check.exe --design-configure $py 'C:\Program Files\LibreOffice\program\soffice.exe'
./target/release/vmt-agent-check.exe --skills-import ./skills
```

The normal saved context is already 16,384. For an already open older process,
restart or use Settings → Models & execution to load the updated setting. Execution
still requires per-action review. Do not run setup against guessed archives.

```powershell
cargo fmt --all -- --check
./build.ps1 -Action check
./build.ps1 -Action test
./build.ps1 -Action clippy -CargoArguments @('--all-targets','--','-D','warnings')
./build.ps1 -Action build -Release
& $py -B scripts/test_design_runner.py -v
& $py -B scripts/creative_benchmark.py --output .local/creative-benchmark-NEW
& $py -B scripts/accept_creative_engine.py --agent target/release/vmt-agent-check.exe --output 'C:\Users\John Mabona\AppData\Local\VMT\CreativeAcceptance\NEW' --vision-model qwen3.5:4b
git diff --check
```

Each output directory must be new. The reusable acceptance script returns nonzero
for any failed model/operation and retains its receipts; inspect those failures.
The historical final primary receipt intentionally retains two rejected vision
calls. Supplemental compatibility and feedback-loop receipts document the later
successful corrected implementation; historical evidence was not overwritten.

Primary local logs: `.local/acceptance-20261124/final-*.log`,
`skills-final.txt`, `models.json`, `metrics.json` and `broker-final.log`.
Artifacts, runtime settings, model files, original packages and recovery work are
not committed to Git.

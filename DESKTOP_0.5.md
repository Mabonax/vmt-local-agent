# Desktop projects and extended tasks — 0.5.0

This change belongs to `vmt-local-agent`, the Rust/egui desktop application.
The preceding Laravel fixture-workspace milestone lives in the separate
`vmt-institutional-ai` repository and is not the source of this desktop's old
12-step limit.

## Interface and workflows

The two supplied screenshots control the layout: charcoal `#181818` canvas,
`#1c1c1c` sidebar, `#2c2c2c` surfaces, neutral text, narrow navigation rail,
project/chat groups, rounded composer and a Sources card on the right. Settings
is a full page with General, Models & execution, Projects & sources, Archived
chats and Storage. Billing and commercial account menus were not added.

Projects have a UUID, display name, canonical source folder, optional instructions
and bounded references. Chats retain their existing UUID/history, with optional
project association, pin/archive state and a continuation checkpoint. Old JSON
records remain readable. Existing workspace-based chats are grouped locally;
source directories are neither moved nor renamed. A malformed project library
is reported rather than silently overwritten with an empty one.

Project creation, instruction editing, reference addition, chat renaming/moving,
pinning and archiving persist through atomic local JSON replacement. Each project
supports up to 32 references. File references pass through the existing scoped
text reader; notes and HTTPS URLs remain reference data. URL registration does
not resolve/fetch a destination, enable internet tools or approve a request.
The source-context builder is capped at 10 KB and individual file excerpts at
3.5 KB; the runtime's existing context-budget refusal remains authoritative.
Chat mode can answer from supplied reference context while still having no tool
execution. Binary document/image ingestion remains a separate capability.

For smaller windows the right pane becomes a Sources & activity window opened
from the chat header. Controls retain keyboard focus and AccessKit names. Segoe
UI Symbol is a fallback for outline navigation glyphs on Windows.

## Why the previous limit was 12

`Settings::default().max_steps` was 12, the settings control stopped at 30 and
`agent::run_inner` clamped every task to 30. A step consumes an actual tool action
or final-answer attempt, so reads/searches and workflow transitions could exhaust
a small budget before implementation finished. This was an application policy,
not a model/Ollama ceiling.

The default is now 48 steps/15 minutes with explicit settings up to 256
steps/120 minutes. Existing legacy settings with the old default of 12 and no
time-budget field migrate to 48; other saved step choices are preserved. Context
and output defaults stay at 8,192/2,048. More steps do not expand VRAM, increase
context automatically, improve model weights or establish correctness.

Three consecutive identical tool/argument requests pause before the third
execution. Step/time boundaries write a paused journal state and a session
checkpoint containing the bounded objective, execution counts, unresolved tool
names and journal ID. **Continue task** explicitly starts a fresh bounded turn;
it clears pending tool requests and initializes empty file observations. A model
must read current files before proposing edits. This is deliberate continuation,
not exact process suspension, automatic crash recovery or side-effect replay.
Moving a chat to another project clears its old continuation. Normal cancellation
and the existing uncertain-effect journal/reconciliation rules remain in place.
Pending and uncertain side effects also have durable state flags, so trimming
the bounded 256-event activity history cannot erase an earlier uncertainty.

Time checks occur between operations and during the approval wait. A running
model request still has its existing 240-second timeout and a reviewed command
its 120-second timeout; this is not a strict instantaneous wall-clock interrupt.
Step/time limits supplement policy, review, scoped file validation and cancellation.
Every mutation, command and network/MCP request still uses the existing review
broker. Approved host processes retain the Windows user's privileges; there is
no new OS sandbox or automatic approval mode.

## Verification

- Windows library tests: 36 passed. The full Cargo test run also passes the new
  integration case (37 tests total), plus binary/doc targets with zero tests.
- The integration case uses a loopback synthetic Ollama protocol server and real
  temporary files. It verifies 14 distinct completed file reads beyond the old
  default, a three-step pause/checkpoint, repeated-action pausing, reference scope
  denial and legacy-chat deserialization. The Windows accepted socket is explicitly
  switched to blocking mode; this fixed a `WouldBlock` failure in the first full run.
- Additional regression cases cover normal/verbatim Windows folder identity,
  file-picker canonicalization, secret/outside reference rejection and uncertain
  effects surviving long-task event retention.
- Clippy with `--all-targets -- -D warnings` passed.
- Existing explicit runtime and restart acceptance passed, including denial,
  failed/successful commands, concurrent-edit protection, workflow persistence,
  project memory and uncertain-action history without replay.
- Live local `qwen3.5:4b` scenario passed: observed failing Rust test, approved
  scratch-file repair, read-back, passing test and finished workflow. The report
  recorded 13 completed tool actions, two failed attempts and no denied actions;
  elapsed time was 33.9 seconds. Approval automation here is narrowly scoped to
  the existing synthetic acceptance fixture, not available in the product UI.
- Live local `qwen3.5:4b` project-source acceptance passed: Chat answered
  `LANTERN-73` from the attached fixture file with no tools or approvals requested.
- Native UI inspected using an isolated profile at `.local/ui-profile`, covering
  the project/chat shell, Sources, saved chat view, full General settings and chat
  context menu. A synthetic chat was moved to No project through the actual UI;
  persisted JSON independently confirmed an empty workspace, null project ID and
  both messages preserved. No normal-profile chats were moved by these checks.

Build with `./build.ps1 -Action build -Release`; test with `./build.ps1 -Action test`.
The separate `vmt-agent-check --project-context qwen3.5:4b` check uses its own
temporary profile and verifies a source-specific canary answer in Chat mode.
The final packaging run used `cargo rustc` with a separate output path because
the open preview locked `target/release/vmt-local-agent.exe` on Windows. The
normal release build remains available after closing that preview; installed
copies in `dist` do not lock the build output. The 0.4.0 executable is retained.
The final desktop shortcut targets `dist/VMT Local Agent.exe` (version 0.5.0).
Earlier preview executables may remain open in isolated profiles and are not
terminated by the packaging process. The final visual pass increased the
composer's reserved height and removed its separator to keep the status row
inside the window.

The supplied Codex screenshots are visual references, not evidence of feature
parity. Drag-and-drop chat moving, multimodal attachments, automatic execution
resume, encrypted storage and full cross-DPI/mobile acceptance remain outside
this increment. The preview profile is separate from normal `%LOCALAPPDATA%/VMT/LocalAgent`
data and is retained; do not overwrite either profile to combine them.

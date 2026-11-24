# VMT Local Agent 0.6.2

A native Rust desktop coding workbench for the Ollama models installed on this
computer. It provides chat, project inspection, reviewed edits, approved commands,
and optional internet tools. This is a first working release, not a claim of Codex
model quality or full feature parity.

## Unified local integration

The `codex/local-agent-unified-integration` branch incorporates remote tool-pack
commit `58d5f6184baa7927ef7da1db578785a3fe46c4f3` into the existing desktop,
skills, memory, MCP and review broker. It exposes **57 Python host capabilities**
(32 execution tools plus 25 host operations), alongside the existing native tools.
This is local development, not a published release.

Settings → **Tools & artifacts** provides dependency inventory, bounded discovery,
exact action review, fixed PDF workflows, artifact opening/saving/previews,
revision comparisons and persistent knowledge collections. The advanced action
form also supports system diagnostics, model memory estimates and research
citations using their displayed schemas. Every pack execution requires review;
Routine task grants do not approve pack operations. Read-only mode blocks them.

The verified Windows tool interpreter is an isolated Python 3.13 environment at
`%LOCALAPPDATA%\VMT\LocalAgent\tool-python-venv\Scripts\python.exe`.
For a new installation, explicitly create and install it, then configure the
reviewed executable paths (no installation occurs at application startup):

```powershell
$toolEnv = Join-Path $env:LOCALAPPDATA 'VMT\LocalAgent\tool-python-venv'
py -3.13 -m venv $toolEnv
& "$toolEnv\Scripts\python.exe" -m pip install -r tool-packs/office/requirements.txt -r tool-packs/pdf/requirements.txt -r tool-packs/visual/requirements.txt psutil==7.0.0
./build.ps1 -Action build -Release
./target/release/vmt-agent-check.exe --tool-pack-configure "$toolEnv\Scripts\python.exe" 'C:/Program Files/LibreOffice/program/soffice.exe'
./target/release/vmt-local-agent.exe
```

Use `none` instead of the LibreOffice path when unavailable. Select a workspace
in the desktop before refreshing capabilities. Reconfigure after a code or
interpreter update: fingerprints intentionally reject drift. On Linux, create a
Python venv outside the model's workspace, build with Cargo, and configure absolute
Python/LibreOffice paths in the same way. Linux execution has not been accepted
on this Windows workstation.

Current checks: **142 automated test executions pass**, plus eight integrated
acceptance scenarios and two actual review/cancellation checks. Qwen 3.5 4B
completed native document/workflow calls and cited knowledge retrieval at a
16,384-token context. Generated report prose still needs factual review; two
JSON fallback trials failed safely. Do not interpret file creation or rendering
as professional document acceptance. OS filesystem/network sandboxing, reliable
complex autonomous authoring, and real Craft MCP acceptance remain outstanding.

See the [integration audit](docs/audits/LOCAL_AGENT_UNIFIED_INTEGRATION.md) for
commands, evidence, dependency decisions, security boundaries and limitations.

## Open the app

Start Ollama, then open `dist/VMT Local Agent 0.6.2.exe` or the **VMT Local Agent** desktop
shortcut. No OpenAI account or API key is needed for local inference.

1. Choose a model beside the composer. Start with **qwen3.5:4b** for Agent mode.
2. Open a project folder. The file browser respects Git ignore rules and skips
   common dependency/build directories.
3. Choose the workflow beside the message box:
   - **Agent:** the model uses project tools to investigate, edit and verify.
   - **Chat:** ordinary conversation; no project tools or edits.
   - **Edit file:** click a file, choose **Edit this file**, describe a change, then
     review the generated patch. This workflow does not require the model to
     choose or call tools, making it useful for reasoning models such as the local
     DeepSeek R1 distillation.
4. Send with **Ctrl+Enter** or the Send button.
5. Choose **Review actions** or **Routine task** beside the composer. Review mode
   asks for each mutation. Routine mode shares a bounded task grant for small
   edits and workflow progress. **Approve once** covers only the displayed action;
   **Deny** leaves it unexecuted. See the thresholds below.

## Existing Word documents and copying (0.6.2)

For an existing Word CV, ask Agent mode to use Colour Intelligence and Professional
Document Design. It can inspect the DOCX, generate/audit an accent, format a new
DOCX while preserving the source text and other package parts, and render review
images. Use a new output filename; originals are never overwritten by these tools.
The CV preset currently targets plain, English-heading CVs and is not a general
Word layout editor. Human visual review remains necessary.

Responses now support **Copy response** with native clipboard feedback and
**Select text** for keyboard selection. Plain response text is read-only and
supports Ctrl+A/Ctrl+C; right-click offers copying. Native clipboard integration
is enabled in the eframe build. Unresolved tool failures suppress an unverified
model success narrative instead of displaying it beneath a warning.

See [the local-agent CV evaluation and changes](docs/skills/document-workflow-062.md).

## Design skills (0.6.1)

The reviewed Colour Intelligence and Professional Document Design packages now
have a registered `design_tool` adapter: palette/contrast/image extraction,
editable DOCX/PPTX composition and page/slide rendering. Every invocation requires
individual approval, including in Routine mode. The original ZIP files remain
unchanged and are not redistributed in this repository. See
[setup, supported inputs and verification](docs/skills/design-skills.md).

The composer provides baseline editable text, bullets and Word tables. Dense
slides and unsupported layouts are rejected. Rendering creates PNG review files;
the agent must report visual review as pending until the images are inspected.

## Task permissions and portable skills (0.6.0)

Choose **Routine task** in the composer or Settings → General to reduce repeated
approvals for ordinary project work. Review mode remains the default. Each turn
gets a fresh grant scoped to the selected workspace, expiring after 15 minutes:

- Up to 12 small file changes and 64 KiB of changed text in total; each change
  must be at most 16 KiB and remove at most 100 lines.
- Up to 20 workflow transitions. Memory notes and knowledge indexing still ask.
- Sensitive paths, instruction files, dependency manifests, scripts and larger
  edits still ask for review. Existing read-before-edit and stale-file protection apply.
- Commands always ask first. Eligible exact test/check commands offer **Approve
  exact command for this task**, covering up to four uses of that exact string.
  Changed arguments, shell chaining or other commands need a new review. There
  are at most eight granted command uses across the task.
- Internet requests and external MCP tools always need individual review.
  Stop cancels the task; restart/continuation creates fresh grants.

Task grants are permission to run project work with the existing host privileges,
not an OS sandbox. Commands can execute changed project code. Skills, documents,
model text and `allowed-tools` declarations cannot create grants.

Settings → **Skills** can inspect/register a local repository containing
`skills/<name>/SKILL.md` (or a direct skills directory), inspect instructions,
enable/disable packages, review updates and unregister sources without deleting
their files. Versions are pinned by a SHA-256 inventory of the manifest and all
supporting files. Any drift blocks loading until reviewed again.

The supplied `vmt-agent-skills` library contains 18 VMT guidance packages. It is
registered by reference, not copied into this app's source. `load_skill` loads
instructions and `skill_resource` loads only reviewed supporting text on demand.
Scripts/assets are inventoried; scripts never execute automatically. Compatibility,
licences and dependencies are displayed separately from actual execution proof.

CLI alternatives after building `vmt-agent-check`:

```powershell
./target/release/vmt-agent-check.exe --skills-import C:/path/to/vmt-agent-skills
./target/release/vmt-agent-check.exe --skills-audit
./target/release/vmt-agent-check.exe --permissions-skills qwen3.5:4b C:/path/to/vmt-agent-skills
```

The final command creates its own temporary profile/project; only the explicitly
requested import changes the normal profile. Read [integration and verification](docs/skills/desktop-integration.md).

The activity panel records actual tool execution. The agent's final answer also
includes execution counts. A model statement is not evidence that work occurred.

## What it can do

- Discover installed local chat models and switch between them.
- Browse, read and search selected-project text files.
- Load bounded root/ancestor and scoped `AGENTS.md` instructions.
- Propose exact text edits or new/replacement files with a reviewable diff.
- Recheck file contents before applying an approved change; preserve a file that
  changed after it was read/reviewed.
- Run a reviewed PowerShell command in the selected project, with bounded output
  and a 120-second limit.
- Fetch reviewed public HTTPS pages and retain source URLs.
- Search the web through Ollama's web search API when configured.
- Save conversations and preferences locally; resume recent conversations.
- Stop a task and prevent further actions. During a model request, stopping takes
  effect when the next response chunk arrives, subject to its 240-second timeout.

## Model compatibility

Agent mode uses Ollama native function calling when the model reports support.
Settings also allow forcing native tools or a constrained JSON fallback. Native
tool calls are processed sequentially through the same review broker.

Not every local model can reliably operate tools. **qwen3.5:4b** passed the live
read → approved edit → read-back acceptance task. The installed
**deepseek-r1-local:7b** did not reliably call tools in early Agent-mode checks;
use Chat or the guided Edit file workflow for that model. Model-generated code
still needs project-specific review and tests.

Default context is 8192 tokens, output limit 2048, and the agent budget is 48 steps
and 15 minutes. Settings → Models & execution supports 1–256 steps and 1–120
minutes per turn. These are application budgets, not Ollama/model step limits.
Increasing steps does not increase context size or grant additional permissions.
Budget checks happen between operations (and while waiting for review); an
in-flight model request or approved command can finish before the pause takes effect.
Three consecutive identical tool actions pause a stalled turn before the third
action executes. A saved **Continue task** checkpoint starts another bounded turn
with fresh file observations; it never automatically replays approved side effects.
Increase context separately only as needed; larger context consumes more GPU/RAM.
Long files, conversations and instructions are bounded. A context error or an
incomplete model response stops the turn before executing any partial action.

## Projects, chats and sources (0.5.0)

The charcoal desktop shell follows the supplied Codex layout: navigation rail,
project/chat sidebar, centered conversation/composer, Sources/Activity panel and a
full settings page. It has no billing or subscription area.

- Create a project with **+** beside Projects and choose its existing source folder.
  Give it a name and optional project instructions. Existing chats are grouped
  from their saved workspace folders without moving source files.
- Start **New chat in project**, search the sidebar or right-click a chat to
  rename, move, pin or archive it. Restore archived chats in Settings.
- In Sources, add a permitted project text file, a note or an HTTPS link. File
  references are reread at the start of Agent and Chat turns; notes and links are
  supplied as bounded context. Links are not automatically fetched. Binary PDF,
  image and Office attachment ingestion is not implemented by this reference UI.
- Moving a chat preserves messages and changes the folder used by future tasks.
  Its saved task continuation is cleared. Moving a chat out of a project removes
  its workspace; select Chat mode for conversation without project tools.
- Project files and reviews remain available; protected files, links/junctions
  and paths outside the chosen root are still refused. References are data and
  cannot grant permissions. There is no full-access/automatic-approval switch.

Projects, references and checkpoints share the existing local plaintext profile.
For a separate test profile use `--data-dir <folder>`; it is not automatically
merged into the normal profile. See [0.5 verification and limits](DESKTOP_0.5.md).

## Skills and persistent project memory (0.2.0 preview)

Agent mode can now call `list_skills` and `load_skill` for bounded local
Markdown workflow guidance. Built-in skills cover Rust development, code review
and project discovery. Additional user-managed `*.md` skills can be installed in
`%LOCALAPPDATA%/VMT/LocalAgent/skills/`. A skill never executes code by itself.

The agent can query historical per-project records with `memory_search`.
Successful reviewed file changes, command completions and web requests record
minimal metadata, not command text or full output. The `remember_project`
tool proposes an explicit short project note requiring **Approve once**.
Memory persists across app restarts in
`%LOCALAPPDATA%/VMT/LocalAgent/memory/`; it is plain JSON and is not encrypted.
Historical notes are reference data, not source-of-truth.

See [Skills and Project Memory](SKILLS_AND_MEMORY.md) for limits, install
instructions and the workstation acceptance checklist. This feature branch
still requires the Windows build, tests and live model/UI verification.

## Extended capabilities (0.3.0 preview)

The next optional layer provides:

- **Document-generation skills:** WordCraft, PdfCraft, GridCraft and DeckCraft
  workflows, using real tools discovered through registered local MCP servers.
  Binary output is available only when compatible native tools are installed.
- **Structured workflows:** persistent per-project `feature`, `bugfix`,
  `refactor` and `document` stages, each transition requiring user review.
- **Local semantic retrieval:** user-approved indexing with an installed Ollama
  embeddings model; limited vector search over plaintext project snippets.
- **MCP stdio:** reviewed discovery and tool calls for registered local servers,
  including registrations for 15 optional ArtCraft Craft CLIs. External executables are not
  OS-sandboxed. This preview supports the MCP 2025-11-25 and 2025-06-18 stdio lifecycles,
  not the newer 2026-07-28 stateless protocol.

Read [MCP and ArtCraft](MCP_AND_CRAFT.md) and
[Knowledge and Workflows](KNOWLEDGE_AND_WORKFLOWS.md) before enabling these
capabilities. The new feature code requires workstation compilation and
acceptance testing; the documentation does not claim verified exports or
live integration of the fifteen native applications.

## Internet

Internet tools are off initially. Enable **reviewed internet tools** in Settings
when wanted. Every request still pauses for review.

- `web_fetch` needs no API key. It accepts public HTTPS port 443, pins resolved
  public addresses, refuses private/loopback/Tailscale addresses, and does not
  automatically follow redirects. Large/binary pages are rejected.
- `web_search` requires an **Ollama** account/API key in `OLLAMA_API_KEY` in the
  environment used to launch the app. The approved query is sent to
  `https://ollama.com/api/web_search`. The key is never saved in app settings.
- Retrieved content is treated as untrusted reference data, not instructions.
- Local inference remains local. Web requests intentionally contact the reviewed
  destination; avoid putting confidential code or personal information in queries.

See [Ollama web search](https://docs.ollama.com/capabilities/web-search).

## Boundaries and local data

File tools reject parent traversal, absolute/drive/stream/device paths, links and
junctions, `.git`, known secret filenames such as `.env`, and common key files.
This is a filename/path guard, not a guarantee of finding all secrets in arbitrary
source code. Example environment files ending in `.example` are readable.

**Approved shell commands run with your Windows account. They are not an OS
sandbox and may have effects beyond the project.** Read the complete command
before approving. The app does not auto-approve, publish, deploy or send messages
on your behalf. There is no autonomous background agent after a turn finishes.

App data lives in `%LOCALAPPDATA%/VMT/LocalAgent/`:

- `settings.json`: endpoint, model and preferences; no API keys.
- `sessions/*.json`: user messages and final assistant responses.

Tool outputs and model reasoning are not saved as session logs. Conversation text
can contain whatever you type or the model quotes; protect local backups normally.
The model endpoint is restricted to loopback HTTP and model metadata must identify
local weights. Cloud model tags are excluded from discovery.

## Build and verify on this workstation

Rust was installed through official Rustup. The build script uses the installed
Visual Studio **2022** C++ toolchain because an incomplete newer Visual Studio
installation otherwise wins Rust's automatic linker selection.

```powershell
.\build.ps1 -Action check
.\build.ps1 -Action test -Library
.\build.ps1 -Action clippy
.\build.ps1 -Action build -Release

.\target\release\vmt-agent-check.exe qwen3.5:4b
.\target\release\vmt-agent-check.exe --edit deepseek-r1-local:7b
.\target\release\vmt-agent-check.exe --tools
```

Live model checks use a temporary project and automatically approve only one
specific expected edit to `hello.txt`. They refuse other actions and verify the
actual saved file. `--tools` explicitly exercises a harmless approved command and
fetches the public `https://example.com/` page.

Keep `Cargo.lock`. Source modules separate the desktop UI, inference transport,
agent loop, review broker, workspace guards and local persistence. No Laravel
repository or deployment is replaced by this application.

## Current scope

This release has no OS-level isolation, terminal emulator, language server,
plugin marketplace, git worktree manager, multi-agent execution, file
deletion tool, background service or app updater. Markdown support is intentionally
basic; fenced code blocks can be copied. Rust source and build scripts are included
so the workbench can be extended after real project usage.

## 0.4.0 workstation verification

See [VERIFICATION_0.4.md](VERIFICATION_0.4.md) for the current build, model,
desktop, MCP, retrieval and research results and remaining release limits.
Earlier verification files describe earlier releases.

After building, explicit acceptance checks can be reproduced with
`scripts/verify-0.4.ps1 -Action runtime`, then `-Action resume`, `-Action rag`,
`-Action research`, `-Action craft`, or `-Action scenario -Model qwen3.5:4b`.
These checks use synthetic files under `.local/verification-04` and an isolated
app-data directory. Only the scenario runs reviewed Cargo commands. Craft
checks launch installed headless servers with roots pointing at test folders;
they do not register servers in the user's normal application data.

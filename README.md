# VMT Local Agent

A native Rust desktop coding workbench for the Ollama models installed on this
computer. It provides chat, project inspection, reviewed edits, approved commands,
and optional internet tools. This is a first working release, not a claim of Codex
model quality or full feature parity.

## Open the app

Start Ollama, then open `dist/VMT Local Agent.exe` or the **VMT Local Agent** desktop
shortcut. No OpenAI account or API key is needed for local inference.

1. Choose a model at the top. Start with **qwen3.5:4b** for Agent mode.
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
5. Review every proposed edit, command or web request. **Approve once** runs only
   the displayed action; **Deny** leaves it unexecuted.

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

Default context is 8192 tokens, output limit 2048, and maximum agent steps 12.
Increase these in Settings only as needed; larger context consumes more GPU/RAM.
Long files, conversations and instructions are bounded. A context error or an
incomplete model response stops the turn before executing any partial action.

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

This release has no OS-level isolation, terminal emulator, language server, MCP
client, plugin marketplace, git worktree manager, multi-agent execution, file
deletion tool, background service or app updater. Markdown support is intentionally
basic; fenced code blocks can be copied. Rust source and build scripts are included
so the workbench can be extended after real project usage.

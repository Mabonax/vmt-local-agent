# Local verification: 19 November 2026

Release: **VMT Local Agent 0.1.0**, native Windows Rust/egui app.

## Passed

- Release build with Rust 1.100.0 and the installed Visual Studio 2022 tools.
- Nine library tests: protected paths, concurrent file changes, no-clobber writes,
  read-before-edit, read-only commands, cancellation before execution, local model
  endpoints, incomplete/unknown actions, and reviewed web preparation/network guards.
- Clippy with `-D warnings`, Rust formatting check and Git whitespace check.
- Real **qwen3.5:4b** native agent: read a temporary `hello.txt`, propose the exact
  replacement, wait for the acceptance harness's tightly scoped approval, write
  it, read it back and finish with three actual tool actions and one file change.
- Real **deepseek-r1-local:7b** guided Edit file workflow: generate an exact patch,
  wait for review, apply the expected patch and read back the actual file.
- Explicit command transport check: reviewed `Write-Output 'VMT_COMMAND_OK'` in a
  disposable workspace, successful exit and expected captured output.
- Public HTTPS retrieval: reviewed `https://example.com/`, expected source text
  and source URL, without an API key.
- Packaged desktop UI: model discovery, text entry, Send, a real read-only task,
  returned file contents, activity records and a subsequent reviewed edit.
- While the approval modal was open, an independent disk read confirmed that the
  fixture still contained `Hello world`. Only after clicking **Approve once** did
  it become `Hello desktop`; the app then read it back and reported completion.
- The chat composer and approval modal were visually inspected on the actual
  desktop. The initial window height/composer were adjusted to keep Send visible.
- Desktop shortcut points to the packaged executable in `dist`.

The live checks used disposable/synthetic files. No clinical, institutional or
other private project data was used for model acceptance.

## Compatibility findings and limits

- The DeepSeek R1 distillation did not reliably call tools in the initial generic
  JSON/native agent experiments. The failed checks were retained under
  `.local/reports`. It is available for Chat and passed the guided Edit file check.
- Qwen's successful native tool workflow does not establish general coding
  correctness, large-repository performance or Codex feature/model parity.
- The key-backed web search integration is implemented but no authenticated live
  search acceptance was run. It needs `OLLAMA_API_KEY` at application launch.
- Commands require review and execute as the Windows user, with no OS sandbox.
- No production deployment, publishing, MCP integration, updater, background
  agent, concurrent-model benchmark or confidential-data acceptance was performed.

Reports: `.local/reports/qwen-native-live.txt`,
`.local/reports/deepseek-guided-edit.txt`, `.local/reports/tools-live.txt`.
The desktop fixture is `.local/ui-project`; it is separate from real projects.

An earlier unversioned development window was preserved because it contained an
unsent user draft. Use **VMT Local Agent 0.1.0** from the desktop shortcut; close the
earlier test window when its draft is no longer needed.

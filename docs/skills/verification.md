# Desktop 0.6 verification

## Deterministic checks

The full Cargo suite passed **44 tests**: 42 library cases and two integration
cases. Strict Clippy (`--all-targets -- -D warnings`) passed. Formatting and
`git diff --check` passed. CI now runs both integration cases and triggers for
`codex/**` development branch pushes as well as main/pull requests.

Permission cases cover workspace/readonly/expiry boundaries, edit count and byte
limits, destructive changes, workflow quotas, protected paths, exact-command
limits/variants, explicit denial and exclusion of network/persistent-memory
actions. Existing cancellation, policy, concurrent-write and journal tests remain.

The actual broker integration uses a loopback protocol fixture and temporary
files. Routine mode performs two edits with zero file reviews; Review mode asks
twice. A protected instruction-file change is denied once and cannot prompt
again. A declared skill `allowed-tools: run_command` does not prevent the shell
operation from asking for approval. A separately granted Git check executes
three times with one exact-command review. A script with an absolute temporary
sentinel is loaded as text and never executed.

Skill cases cover real multiline YAML, invalid names, duplicate keys, anchors,
unsupported include tags, resource traversal, script/resource drift, activation,
deactivation, persistence/reload and read-only resource use. Existing desktop
integration retains its 14-read execution and paused continuation tests.

The source library's own verifier reports 18 valid local skills and 21 specialist
pins; all 17 management tests passed. Those are source-management checks, not
proof of document/media execution or all domain workflows.

## Live local-model acceptance

`vmt-agent-check --permissions-skills qwen3.5:4b <source>` passed using the actual
local Ollama runtime and supplied VMT library. The independently checked report:

| Observation | Result |
| --- | --- |
| Portable guidance packages registered in isolated profile | 18 |
| VMT feature-delivery skill loaded by model tool call | PASS |
| UI design supporting reference loaded by model tool call | PASS |
| Fixture edit and exact file read-back | PASS |
| Completed Git checks | 2 |
| Command approvals | 1 |
| Other approvals | 0 |
| Runtime errors | 0 |
| Elapsed | 15.57 seconds |

The acceptance helper grants only the exact requested scratch-project Git check;
it is not a product automatic-approval switch. It refuses unrelated proposals.
The normal profile is not used for live fixture execution. A successful guided
case proves this path, not autonomous selection quality across all 18 skills or
parity with Codex. The live report is retained in ignored `.local/verification-06`.

## Native desktop inspection

The packaged Windows 0.6.0 executable was opened with an isolated profile.
Actual screenshots and the accessibility tree confirmed the charcoal chat shell,
Routine task composer control and 48-step/15-minute budget. General settings
displayed the edit, byte, workflow and command-grant limits. The Skills page
displayed 18 registered packages with enabled status, versions, licence status,
content hashes and inspection controls. No permission setting or approval was
changed through UI automation. This is a single-window inspection, not full
display-scaling or accessibility certification.

The normal profile's 18 registered fingerprints were independently compared with
the committed inventory. The desktop shortcut targets the packaged 0.6.0 build;
the prior shortcut was backed up locally. Existing preview processes were retained.

## Limits

The provided broad general-purpose-platform brief is not represented as complete.
This desktop-only increment adds portable guidance and task permissions. It does
not add PDF/DOCX/XLSX/PPTX engines, OCR, transcription, schedulers, new database
connectors, browser automation, an MCP HTTP transport or a Laravel integration.
Existing optional tools retain their own installation and acceptance gaps.

Instruction read/manifest validation, actual tool execution and live agent use
are separate statuses in the inventory. Only the selected VMT skill/resource path
has live-model acceptance here. No source scripts or third-party skill installer
were executed as a consequence of registering the library.

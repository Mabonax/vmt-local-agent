# Enterprise Runtime 0.4.0 — Policy, Journal, Research

This is an **implementation preview**, stacked on 0.3.0, not a certified
enterprise release. Windows build, tests and live acceptance remain required.

## Deterministic permission checking

`src/policy.rs` classifies every registered tool under one of:

- read (no mutation)
- project-write (reviewed edits)
- shell (reviewed commands)
- internet (reviewed web fetch/search)
- external-process (reviewed local MCP)
- local-data-write (reviewed memory, workflows, embedding indexes)

Unknown tool names are rejected. Read-only mode rejects all mutating classes,
including MCP process startup and local memory writes. Internet tools require
a separate enabled Internet setting and approval for each request.

Policy decisions occur in Rust, independent of model messages. The execution
broker crosschecks the proposed operation against the exact approved tool name.

This policy is not an OS sandbox: approved shell/MCP processes can access
resources allowed by the current Windows account. True isolation, host-wide
network egress policy, multiple identities and central governance are future
requirements.

## Durable task journal and recovery

Each Agent-mode task creates a local journal under:

```text
%LOCALAPPDATA%\VMT\LocalAgent\task-journal\<uuid>.json
```

The journal includes task ID, workspace, timestamp, policy results, approvals
and minimal execution outcomes. It excludes prompts, tool arguments, command
bodies, web queries, API keys and full tool outputs.

The execution broker writes a **dispatching / uncertain-until-result** marker
before every approved action. It records the result after the tool returns.

If the program crashes after dispatch but before recording a result, the task
is marked as interrupted/uncertain on later inspection. VMT **does not**
automatically replay any command, edit, web request or MCP call. Users must
inspect actual effects and decide whether to retry.

`task_history` reads task summaries for the current selected project.
`research_sources` shows previously recorded approved HTTPS fetch sources.
This release does **not** implement full SQLite WAL journaling, encrypted
audits, process-level idempotency keys, cross-process transaction locks,
replayable task checkpoints or a persistent scheduler.

A JSON journal is a transition step toward SQLite. Do not market it as
tamper-evident or transactionally equivalent to SQLite.

## Secure internet research

Inference remains local through loopback Ollama. Web requests are separate
and must be approved. An internet operation may transmit public queries to
the configured online provider, or request a public HTTPS page.

The preexisting web fetch controls include:
- HTTPS only, port 443, no URL credentials
- Rejected private, loopback, multicast, special-use and Tailscale IPs
- DNS address validation and pinning for the outbound connection
- No automatic redirects or proxy inheritance
- Bounded responses, timeouts, and allowed text content types
- Untrusted page text marked as reference content, not instructions

New in 0.4:
- Deterministic policy gate before web request approval/execution.
- Public search queries limited to 500 bytes and a single line.
- A *basic* heuristic rejection of common credential-bearing query patterns.
- Source provenance saved per project after a successful approved HTTPS fetch;
  query parameters and URL fragments are omitted from the saved source URL.
- `research_sources` returns the most recent recorded URLs and timestamps.

**Limits and risks:** keyword screening does not detect every credential or
confidential text. An approved query may still disclose sensitive information.
Fetched source text can be false, malicious, stale, or prompt-injected. Source
provenance is metadata, not a proof that a claim is correct. No external search
service should receive private project source code without explicit authority.

### Two ways to research

1. **Ollama web search**: uses `OLLAMA_API_KEY` and calls Ollama's hosted web
   search endpoint. This necessarily transmits the approved public query to
   a remote service. No key is required for local inference itself.
2. **Direct HTTPS web fetch**: calls a specific public URL through the reviewed
   local network client. No Ollama account or key required.

The recommended loop is:

```text
Ask → draft a public research query → policy check → user approval
    → search results → select source → policy/approval → HTTPS fetch
    → record source metadata → compare evidence → cite fetched URLs
```

Before treating research as enterprise-grade, add a research workspace with
grounded claim-to-source mapping, publication and retrieval dates, source
quality indicators, contradictory evidence analysis, and citation export.

## Validation checklist for Codex

```powershell
git fetch origin
git switch feature/0.4.0-policy-journal-research
git pull --ff-only
cargo fmt --all
.\build.ps1 -Action check
.\build.ps1 -Action test -Library
.\build.ps1 -Action clippy
.\build.ps1 -Action build -Release
```

Use only disposable projects and public research content.

1. Check policy for unknown tools, read-only mode, disabled internet and
   MCP command restrictions.
2. Approve a harmless file edit; inspect the journal for the approved
   action and completed result.
3. Deny a different edit; confirm no mutation and a denial record.
4. Simulate interruption and verify it is reported uncertain, never
   automatically replayed.
5. Try malformed or credential-bearing public queries, and confirm they
   fail before any network operation.
6. Fetch a public HTTPS page and verify the saved source URL excludes any
   query parameters or fragments.
7. Verify task history and research ledger separation across projects.
8. Exercise Qwen native Agent mode, DeepSeek guided edit, and real MCP tools
   with approval and cancellation tests.
9. Verify the actual exe UI, tests, Clippy, formatting and offline behavior.

Keep #4 in draft until these are verified. Merge order: #1 → #2 → #3 → #4.

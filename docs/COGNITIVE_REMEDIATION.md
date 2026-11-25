# Cognitive remediation — bounded implementation status

The development branch adds narrow task contracts, evidence excerpts, an
opt-in exact-evidence text report, exact monetary reconciliation, simple
DOCX text replacement, competency hints, invocation normalization, and
fixed PDF/Word workflows. These are not general semantic fact checking,
arbitrary document editing, or fully autonomous workflow execution.

Model profiling uses the existing local Ollama show response and retains
the explicit native/JSON preference. Reported capability is not observed
reliability. Native commands now inherit an explicit environment allowlist
and Unix cancellation targets the process group. This may require explicit
configuration for custom build environments. It does not sandbox file or
network access, prevent credential reads from disk, or contain detached
descendants. MCP and creative runtimes need additional isolation work.

## Acceptance matrix (not yet executed against local models)

- Qwen 3.5 4B native and JSON modes: produce a real DOCX from supplied facts.
- Evidence-only report: reject invalid references and invented prose.
- Financial total: 0.10 + 0.20 must equal 0.30 exactly; detect mismatches.
- DOCX edit: preserve tables/header and reject ambiguous match count.
- Word workflow: actual artifact, structure and hash; deny missing child tools.
- Reject unregistered tool, malformed arguments and denied-action replay.
- Cancellation: check descendant processes on Windows and Linux.
- Run visual review of generated Office and PDF pages.
- Record model name, quantization, context, duration, approvals, failures,
  artifact hashes and acceptance outcomes without private source contents.

Do not claim real-model acceptance or production readiness from unit tests.

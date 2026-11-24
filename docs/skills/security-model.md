# Permission and skill trust boundaries

`task_permissions.rs` receives operator-selected mode, the canonical workspace
and prepared proposals. Model output, skill files and tool results cannot set
the mode or issue a grant. Grants live in one runtime invocation; they are not
stored in chat checkpoints or shared with another workspace/process/turn.

Routine eligibility is conservative. Dot paths, parent traversal, instruction
files, permission/security modules, dependency manifests, configuration,
authentication/policy/migration/script directories, shell/binary/key files,
empty replacements and oversized/destructive changes go through review or the
existing hard denial. The implementation's exact path list is authoritative;
this is a risk threshold, not a proof that all source changes are harmless.

Each reuse consumes quota before dispatch and is recorded in the durable task
journal. All execution still passes the existing policy check, cancellation
check, proposal/tool match and file concurrency protection. Failure consumes the
attempt's grant quota. Explicit manual approvals can authorize further actions;
the task grant itself cannot renew or elevate its limits.

Command reuse is keyed by a SHA-256 fingerprint of the exact prepared command and
tool, bound to the invocation's workspace and lifetime. An explicit first review
creates at most three subsequent uses. Skill declarations cannot create those
records. Expired or mismatched commands prompt again. Network requests, MCP
processes, persistent memory and indexing are excluded from routine grants.

Portable sources are selected by the operator. Registration hashes the manifest
and every allowed supporting file, with bounds and link/junction/secret-path
checks. Loading rechecks the complete package fingerprint and then the requested
file's hash. Changed, disabled, missing or unregistered resources fail closed.
Resources cannot traverse outside their reviewed package. Text is reference
data; scripts are never automatically launched. Updating a source is an explicit
operator review, not an automatic pull of upstream HEAD.

This is a local desktop trust model. Local profile JSON is plaintext and protected
by the Windows account/filesystem, without cryptographic signatures, encryption
or cross-process locking. An attacker with write access to the operator profile
or control of the executable/runtime is outside this boundary. Approved commands
and MCP servers retain host privileges; task grants do not add an OS sandbox.

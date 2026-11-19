# Security and execution boundaries

VMT Local Agent 0.1.x uses per-action human review. It is **not** a security
sandbox and must not be used to run untrusted commands on a machine containing
sensitive credentials or valuable data.

## Command execution

- The app shows the exact shell command and requires approval for that action.
- Approved commands inherit the Windows user's permissions. The selected project
  is only the *working directory*, not a filesystem or network restriction.
- Some known API-key environment variables are stripped, but this does not
  protect other environment variables, files, credentials, or OS services.
- A timeout or cancellation may leave effects behind. Commands can launch child
  processes and can change files before they stop.
- Non-zero command exits and timeouts are reported as tool failures, not successes.
- Prefer disposable workspaces and a separate low-privilege Windows user when
  testing unfamiliar scripts or model-generated commands.

## Filesystem tools

Dedicated file tools restrict paths, reject known secret names, and require review.
They compare the expected prior file content before replacement and revalidate
paths before persistence. These checks reduce accidental overwrites and common
path attacks; they are **not** an OS-level transaction or a guarantee against
concurrent path replacement and reparse-point races.

Do not rely on these guards for adversarial multi-user workspaces. A future
hardened file broker should use handle-relative operations and OS-enforced
isolation.

## Model and retrieval content

Project files, AGENTS.md, commands, and retrieved web pages may contain
prompt-injection attempts. The runtime's security policies and user approvals
take precedence over instructions from untrusted content. Review web destinations
and queries before approval.

## Validation

The PR CI runs Windows compilation, formatting, library tests and Clippy.
Live Ollama model, GUI approval, and network acceptance still require explicit
manual checks on a test workstation. A green CI build is not a penetration test.

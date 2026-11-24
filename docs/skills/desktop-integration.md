# Desktop permissions and portable skills — 0.6.0

## Use

Open the updated **VMT Local Agent** shortcut. Choose **Routine task** beside the
composer for ordinary project work. **Review actions** remains the default, and
Read-only mode remains an overriding restriction. The selection and limits are
visible before Send; every turn/continuation initializes a new in-memory grant.

| Action | Routine task behavior |
| --- | --- |
| Read/search/skill guidance | Existing read permissions |
| Small project edits | Up to 12 edits, 64 KiB changed text total, 16 KiB/change, at most 100 deleted lines/change |
| Workflow progress | Up to 20 transitions |
| Sensitive paths or larger changes | Exact-action review |
| Test/check command | First review required; eligible exact strings can receive four uses, eight granted command uses/task maximum |
| Internet, MCP, memory notes, indexing | Individual review |

Grants expire after 15 minutes, even if a longer execution time is configured.
Quota/expiry boundaries return to review; they do not silently expand permission.
Stop cancels the running task; queued actions still pass cancellation/policy
checks. A denied exact proposal is not prompted again during that turn.

Only these exact command strings can receive repeat grants:

```text
cargo test --offline
cargo fmt --all -- --check
cargo clippy --offline --all-targets -- -D warnings
git status --short
git diff --check
```

Arguments, whitespace changes, chaining, arbitrary shell/Python commands and
other commands do not match. A grant covers execution of current project code
with the Windows user's privileges; it is not process isolation. The first
review explicitly offers either **Approve once** or **Approve exact command for
this task**. There is no universal full-access setting.

## Skills management

Settings → **Skills** can choose a repository or a direct skills directory,
inspect packages/resources, approve a pinned version, enable/disable skills,
review source updates and unregister a source without deleting its files.
Refresh checks displayed status; runtime loading independently checks every
version before returning content.

The supplied `vmt-agent-skills` checkout contains 18 local VMT guidance packages
at commit `d24b9efe2ee00a3678dcdf66968f37eb40f3e861`. They are registered by
reference in `skill-libraries.json` in the active local profile. Source folders
are not copied or modified. The machine-readable [inventory](inventory.json)
records manifest versions, content fingerprints and verification boundaries.

`list_skills` supplies concise descriptions. `load_skill` returns selected
instructions and an inventory of supporting resources. `skill_resource` reads
one exact reviewed resource. Scripts are returned only as text; neither a skill
nor its `allowed-tools` field can authorize execution. Existing registered tools
and the review/permission broker remain the only execution path.

## Format and limits

The loader follows the [Agent Skills specification](https://agentskills.io/specification)
for folder/name matching, required YAML name/description, optional licence,
compatibility, metadata, allowed-tools and progressive disclosure. It implements
a deliberately bounded YAML subset: aliases/anchors, custom tags, includes,
property interpolation, duplicate keys and excessive nesting are rejected.

The pinned YAML parser is `serde-saphyr 2.2.0` with deserialize-only features;
fingerprints use `sha2 0.10.9`. Both declare MIT OR Apache-2.0 licensing. Resolved
dependencies and checksums are in Cargo.lock. Optional YAML include/interpolation
features are not enabled.

Bounds: 8 libraries; 128 skills/source; 64 KiB/manifests; 128 files and 128
directories/package; 512 KiB/resource; 4 MiB/package. Main instructions are
disclosed up to 16 KiB and individual text resources up to 12 KiB. Binary assets
are inventoried but not decoded as model text. Existing context limits still
apply. Unsupported or oversized packages fail registration/loading.

The VMT manifests do not declare a licence; the UI reports **not declared**.
This integration does not redistribute those bodies or certify rights in
separately referenced packages. Dependencies mentioned by guidance (Context7,
Flutter tooling, browsers, document engines, etc.) must be installed/configured
and verified separately. Registered guidance is not executable-capability proof.

## Commands and acceptance

```powershell
./build.ps1 -Action test
./build.ps1 -Action clippy -CargoArguments @('--all-targets','--','-D','warnings')
./build.ps1 -Action build -Release -Binary vmt-agent-check
./target/release/vmt-agent-check.exe --skills-import C:/path/to/vmt-agent-skills
./target/release/vmt-agent-check.exe --skills-audit
./target/release/vmt-agent-check.exe --permissions-skills qwen3.5:4b C:/path/to/vmt-agent-skills
```

The explicit import updates only the active local profile; it does not install
dependencies or execute downloaded scripts. The live check uses its own
temporary profile/project. Its report is
`.local/verification-06/permissions-skills-live.json` and contains counters and
outcomes rather than raw project/source contents. See [verification](verification.md).

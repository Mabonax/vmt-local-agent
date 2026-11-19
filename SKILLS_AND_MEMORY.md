# Skills and Project Memory (0.2.0 preview)

This release adds **local skill discovery and persistent project memory** to Agent mode.
Both operate without a cloud service, database server, or additional dependency.

## Skills

Use `list_skills` to discover available skills, then `load_skill` with the
exact ID to retrieve their instructions. A skill is **reference guidance**, not
a script or an executable plugin. Tool execution still follows the existing
review broker and workspace restrictions.

Three initial built-in skills are included:
- `project-discovery`: inspect repository conventions before modifying code
- `rust-development`: conservative Rust implementation and verification
- `code-review`: source-backed code review with test recommendations

You can add Markdown files at:

```text
%LOCALAPPDATA%\VMT\LocalAgent\skills\laravel-development.md
%LOCALAPPDATA%\VMT\LocalAgent\skills\pdf-generation.md
```

IDs must contain only lowercase letters, digits and hyphens (maximum 64
characters). Each file must be UTF-8 Markdown, under 16 KiB, and a regular
non-symlinked file. Up to 32 local files are indexed, ordered by filename.
Bundled skills cannot be overridden by local skills with the same ID.

For example, `laravel-development.md`:

```markdown
# Laravel development
Purpose: Extend an existing Laravel project without changing unrelated features.

1. Read composer.json, current routes, models and existing tests.
2. Follow installed Laravel and project conventions.
3. Propose a minimal change through the reviewed file tools.
4. Run tests only after the user approves the proposed command.
5. Report actual test results and any unverified assumptions.
```

Project and internet content are untrusted. Skills cannot disable approvals or
override application policies. An installed skill can still mislead the model;
review all proposed actions.

## Persistent project memory

Memory is saved separately from the project under:

```text
%LOCALAPPDATA%\VMT\LocalAgent\memory\<workspace-key>.json
```

Records are associated with the canonical project path. Each record file
contains no command body, web query, tool output, reasoning trace, or automatic
conversation transcript. Only the following are saved:

- Metadata for successful reviewed file changes (path only), commands (no command
  text) and web requests (no URL/query).
- Explicit project notes created with `remember_project`, each requiring the
  user's individual approval.

Use `memory_search` with a short phrase to retrieve matching notes and
historical verified-action records. The agent also receives a small amount of
recent project memory at the start of Agent mode. **History is not current source
truth**: recheck actual repository files before making edits.

Storage limits: 200 events and 50 notes per project; each note at most 500
UTF-8 bytes. Search responses and context injection are bounded. Memory is
plain JSON, not encrypted, so do not save passwords, secrets or sensitive
personal details. Backups of the user's profile may include memory files.

The current memory lookup uses plain text matching, **not semantic embeddings**.
There is no background indexer, autonomous learning or model fine-tuning.

## Validation for workstation Codex

After pulling the stacked branches, run:

```powershell
.\build.ps1 -Action check
.\build.ps1 -Action test -Library
.\build.ps1 -Action clippy
.\build.ps1 -Action build -Release
```

Use a disposable project to verify:

1. `list_skills` and `load_skill` return bounded skill content.
2. `remember_project` pauses for review; Deny writes nothing.
3. Approve a harmless note, restart the app, and retrieve it with
   `memory_search` on the same project.
4. Complete a reviewed file edit and verify a historical file-change record.
5. Check that Read-only mode rejects `remember_project`.
6. Switch projects and verify the previous project's note does not appear.
7. Repeat with Qwen native tools and DeepSeek's guided file edit; the latter
   records successful file changes but does not have independent memory tool use.

No Windows desktop or real-model acceptance is claimed until these pass.

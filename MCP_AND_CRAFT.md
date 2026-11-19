# Local MCP + ArtCraft integration (0.3.0 preview)

VMT Local Agent supports explicitly reviewed **local stdio MCP** tool discovery and
calls. This implementation targets the negotiated MCP **2025-11-25** lifecycle.
It does **not** yet support the newer 2026-07-28 stateless MCP protocol,
Streamable HTTP, OAuth, MCP resources/prompts, or MCP Apps UI.

## Security model

- The client does NOT install, download, update, or start an MCP executable on launch.
- `mcp_servers` lists registered servers without launching them.
- `mcp_tools` and `mcp_call` each require explicit **Approve once** before
  starting or interacting with an external process. The dialog displays the
  executable, startup arguments, server, tool name and JSON arguments.
- A model must call `mcp_tools` before it may call a tool from that server.
  Undiscovered tool names are rejected.
- The approved external MCP process runs **with your Windows account**; it is
  NOT sandboxed by the project's file path guards. The runtime passes only a
  minimal allowlist of Windows environment variables, no API-key variables.
- Each session remains active during one agent task so native creative engines
  can keep project/document state between calls. Sessions are terminated when
  the agent task ends, fails or is cancelled. There is no background service.
- Review server binaries, startup flags, tool arguments and output before use.
  MCP responses and tool descriptions are untrusted reference data.
- MCP responses are limited and will not display arbitrary large binary media.
  Some native apps require more complex session/streaming support than this
  initial transport provides; validate each app before calling it supported.
- A command may have effects before a timeout or cancellation. There are no
  cross-application transactions or guaranteed rollback.

## ArtCraft applications

The upstream [ArtCraft organization](https://github.com/storytold) publishes the
native Craft applications. The independent
[Devsolux Storytold integration](https://github.com/devsolux/storytold) documents
the fifteen native CLI MCP servers and the correct launch flags.

| Group | Native app CLI |
| --- | --- |
| Photo editing | photocraft |
| Vector illustration | vectorcraft |
| Film/video editing | filmcraft (user-wide) |
| RAW photo development | lightcraft (user-wide) |
| PDF handling | pdfcraft |
| Motion / effects | effectcraft |
| Layout / publishing | designcraft |
| Spreadsheets | gridcraft (user-wide) |
| Word processing | wordcraft |
| Presentations | deckcraft |
| CAD | cadcraft (user-wide) |
| Audio | soundcraft |
| Interface design | figcraft |
| Materials | materialcraft |
| 3D textures | texturecraft |

The four user-wide apps (FilmCraft, LightCraft, GridCraft, CADCraft) may access
anything the current Windows user can access. Even the eleven root-flagged apps
are **not OS-sandboxed**.

### Installing binaries

Install and verify the actual Craft CLIs first. The recommended upstream helper
is in `devsolux/storytold`; review its manifest, verification and setup
instructions before running it:

```powershell
python storytold.py setup --all
```

The upstream tool installs only root-flagged apps without the additional
unconfined-access acknowledgement. To include the four user-wide apps, its
setup requires `--acknowledge-unconfined`. VMT neither downloads nor
validates those third-party binaries itself.

### Registering with VMT

From this repository, after the native binaries have been installed to
`%LOCALAPPDATA%\Storytold\craft\bin\`:

```powershell
.\scripts\register-artcraft-mcp.ps1
```

This creates `%LOCALAPPDATA%\VMT\LocalAgent\mcp-servers.json` with
existing root-flagged Craft CLIs only, using a dedicated workspace at
`%USERPROFILE%\craft-work-storytold\` and an `outputs\` directory.

To opt in to the four user-wide executables, run:

```powershell
.\scripts\register-artcraft-mcp.ps1 -IncludeUnconfined -Force
```

**Review the existing registry before `-Force`, which replaces it.** The
script never starts or installs executables. It writes a local JSON
configuration and creates workspace folders.

A third-party server can also be registered manually:

```json
{
  "servers": [
    {
      "id": "wordcraft",
      "command": "C:\\Users\\YOU\\AppData\\Local\\Storytold\\craft\\bin\\wordcraft-cli.exe",
      "args": [
        "mcp",
        "--automation-read-root", "C:\\Users\\YOU\\craft-work-storytold",
        "--automation-write-root", "C:\\Users\\YOU\\craft-work-storytold\\outputs"
      ]
    }
  ]
}
```

Use actual absolute executable and root paths, not placeholders. The schema
is a list of `servers` with `id`, `command` and `args`; VMT does not
inherit Codex or Claude Code MCP registrations automatically.

### Expected workflow

1. Open a disposable Craft project and load the `artcraft-suite` skill.
2. Call `mcp_servers` to find an installed app.
3. Call `mcp_tools` for the chosen server and approve process startup.
4. Read the returned real tool definitions and schemas.
5. Call `mcp_call` with the actual tool name and JSON parameters; approve each
   call, preferably using staged copies.
6. Export to the approved outputs directory, reopen it through the native app,
   and check the durable file exists. In-memory tool state alone is not a file.
7. Finish the task; the MCP process is stopped automatically.

WordCraft, PdfCraft, GridCraft and DeckCraft power document generation when
installed. Without them, the agent may prepare a reviewed Markdown/CSV fallback,
but must not claim to have produced DOCX, PDF, XLSX or PPTX files.

## Acceptance required on Windows

- Confirm the registry can list all installed servers without launching them.
- Confirm **Deny** on MCP discovery or invocation starts no process/call.
- Confirm the approval UI displays the actual executable path and flags.
- Confirm invalid/undiscovered tool names fail closed.
- Verify WordCraft multi-step open/edit/export in a single Agent task.
- Verify PdfCraft and DeckCraft similarly with disposable documents.
- Confirm user-wide apps are excluded unless explicitly registered.
- Confirm Stop closes live MCP processes and does not auto-resume a task.
- Verify no API key or sensitive environment variable is inherited.
- Verify a CLI that supports only newer MCP protocol fails with a clear error.

See `SECURITY.md`; approval is never equivalent to OS-level isolation.

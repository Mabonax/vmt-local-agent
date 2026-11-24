# Desktop capability audit — 24 November 2026

Scope was confirmed as **desktop agent only**. The supplied broad skills brief
names the Laravel platform, but no Laravel orchestration or authentication code
is changed in this increment. No new application replaces either runtime.

The branch starts at `2f46ddd` on the 0.4 development branch. The preceding
desktop/projects work is preserved in commit `c5528c0` before the permission and
portable-skill changes. The desktop baseline passed 37 tests. The separate VMT
skills repository passed its static verifier (18 local packages, 21 third-party
pins) and all 17 management fixtures; it was not modified.

| Component | Existing behavior | Reused/extended |
| --- | --- | --- |
| Runtime | Native Ollama functions or validated JSON actions; sequential bounded loop | Same runtime and tool execution boundary |
| File tools | Selected-root paths, protected files, read-before-write, concurrent-change checks and atomic replacement | Unchanged; grants do not bypass these checks |
| Review | Boolean per-action approval for every mutation | Typed once/deny/exact-command decisions; bounded task grants |
| Commands | Reviewed host processes; 120-second timeout and bounded output | Same executor; exact reviewed check-command reuse only |
| Internet | Disabled by default; reviewed public HTTPS fetch/search and SSRF restrictions | No additional network permissions |
| MCP | Registered stdio processes and reviewed tool calls | No automatic MCP grants or new transport claims |
| Skills | Eleven built-ins and bounded flat Markdown guidance | Added portable SKILL.md metadata/resources and reviewed version pins |
| Memory/retrieval | Reviewed project notes, local embeddings and bounded retrieval | No new permanent-memory or indexing grants |
| Workflow/journal | Reviewed stages, durable metadata and uncertain-effect markers | Routine grants can cover bounded progress transitions |
| UI | Projects, chats, Sources and full settings | Task mode selector and Skills management |

No OS sandbox exists for approved host commands/MCP servers. Smaller local models
can still choose incorrect tools or produce incorrect code. The library at the
user-provided path is development guidance, not a collection of installed PDF,
spreadsheet, audio or browser execution engines. Its 21 specialist package pins
are references to separately managed packages; this change does not register or
redistribute them as additional executable skills.

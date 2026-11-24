# Design skill adapters — desktop 0.6.1

The user supplied two ZIP packages: `colour-intelligence` and
`professional-document-design`. Their instructions and references are registered
alongside the existing 18 VMT skills in the normal desktop profile. The loose
`SKILL.md` next to the ZIPs duplicates Colour Intelligence; it is not registered
as a third package. Neither archive declares a version or licence. The package
bodies are installed privately by reference, not redistributed in this Git repo.

| Original ZIP | SHA-256 |
| --- | --- |
| colour-intelligence.zip | `7620aba647a80f2085fe41f859aadb2061c5e816f071cde9989ae24c8f6238ee` |
| professional-document-design.zip | `5ddd40915c7b2b3b4700f5f40f17dbcff9234d8bbacf0402db5db6281fd31f67` |

The registered package fingerprints are
`2f14b6d238323a329e20a1ed8a66660ba148dd1f95e1c3163c9a8511de4482a8`
(colour) and
`9a87d4c301726ef75c432b8050cc1832f872f55377f53893d547a819d23b0bf7`
(documents), including every supporting file.

## Registered operations

Load the relevant skill and reference first. The agent uses `design_tool` with
`operation` and only the arguments for that operation:

| Operation | Arguments | Result |
| --- | --- | --- |
| palette | seed `#RRGGBB`, optional mode and theme | Role tokens, original brand swatch and pairwise contrast checks |
| contrast | foreground/background `#RRGGBB` | Deterministic sRGB contrast ratio and threshold flags |
| extract | input PNG/JPEG/WebP, count 2–16 | Dominant opaque image colours |
| compose | input JSON, output new DOCX/PPTX | Editable native text, bullets and Word tables |
| render | input DOCX/PPTX, output new directory | Every page/slide as PNG, source hash and review.json |

Paths are relative to the selected project. Output parents must already exist;
files/directories must be new. There is no overwrite option. Composition accepts
the supplied title/subtitle/sections contract and rejects unknown fields, dense
slides, slide tables and malformed Word tables. It supports 1–30 sections and
30,000 content characters. Use richer tools for charts, images and custom layouts.

The adapters use the reviewed palette and composition functions. The supplied
render script is replaced by a fixed broker worker that adds bounded inputs,
an isolated LibreOffice profile and rejection of external relationships, active
objects and external-content fields. Word output removes the dependency
template's title border and aligns subtitle/table accents with the supplied
navy/teal standard. Original package files and fingerprints remain unchanged.

## Permission and process boundaries

Each `design_tool` operation is classified as external-process execution and
requires individual review. Routine edits and exact-command repeat grants do not
cover it. Read-only mode blocks it. Review displays exact arguments,
package fingerprint, input hash and runtime executables. The runtime, workspace,
package and input are checked again before dispatch. Disable/unregister/drift
blocks execution. Discovery and resource reads still never execute scripts.

The broker accepts only five fixed operations from these two skill IDs; there
is no arbitrary script path, command, code argument, installation or network
operation. It copies the reviewed script bytes and input into a private invocation
directory. Python uses isolated mode and a small environment, with bounded output,
120-second cancellation/timeout and owned-process-tree cleanup. New artifacts are
published into the selected project only after successful execution. A failed
publication can leave an incomplete new review directory and is reported as a
failure; it must not be treated as a verified deliverable.

These are application controls, **not an OS sandbox**. Reviewed Python packages,
native dependencies and LibreOffice execute with the Windows user's privileges.
Executable pins are not a full dependency/environment attestation. The operator
must maintain the private environment; it is not automatically updated. Existing
filesystem race limitations also apply.

Input JSON is at most 64 KiB; images/Office inputs at most 16 MiB. Image decoding
is restricted to PNG/JPEG/WebP with a 16-million-pixel cap and decompression
warnings treated as failures. Office ZIP expansion is capped at 64 MiB/2,000
entries. Rendering supports at most 40 pages/slides. Files with macros, active
objects, external relationships or external-content fields are unsupported.

Rendered PNGs always leave `visual_review: PENDING`. A text-only local model has
no image-inspection tool in this increment. It must hand off visual review and
cannot certify layout or accessibility merely because rendering passed. Pairwise
contrast flags do not establish complete accessibility or print/ICC accuracy.

## Operator setup

Build `vmt-agent-check` with `build.ps1`. Then run the explicit setup script with
the directory containing the original reviewed ZIPs and the installed local
LibreOffice executable:

```powershell
python scripts/setup_design_skills.py --archives C:/path/to/archives `
  --agent target/release/vmt-agent-check.exe `
  --soffice 'C:/Program Files/LibreOffice/program/soffice.exe' `
  --install-dependencies
```

The setup verifies exact archive hashes and bounded safe extraction, preserves
existing package files, creates a private venv, installs only pinned wheel
dependencies from `scripts/design-requirements.txt`, and registers the packages
and runtime in the normal profile. It never changes task permission preferences.
Omit `--install-dependencies` when the private environment already exists. A
different `--profile` is supported for isolated setup. No installer runs during
app startup or skill discovery.

The CLI `--design-operation '<JSON>' <project>` is an explicit operator execution
command, not a model bypass. It uses the same prepare/execution/journal boundary.
Never pass unreviewed model output into this operator command automatically.

## Verification

- Full Rust suite: 46 passed, including permission, input/script drift,
  no-overwrite, tool/proposal mismatch, readonly and cancellation cases.
- Strict Clippy for all targets passed. Formatting and diff checks passed.
- Original Colour Intelligence tests: all four passed in the private runtime.
- Adapter Python boundary tests: all six passed. CI includes these tests with
  pinned dependencies; private source archives are not required for CI.
- Actual Rust-broker acceptance completed eight operations: black/white contrast
  21:1, light/dark palettes with passing text-pair checks, red/blue extraction,
  native editable Word and PowerPoint creation, and both render operations.
  Independently checked text/table preservation, expected 1 Word page/3 slides,
  artifact hashes and rejection of overwrites.
- Manual image inspection covered the release fixture's Word page and all three
  slide images. Text was readable and complete, without clipping or overlap;
  the Word title rule was removed and table accents rendered in the supplied teal.
- The packaged 0.6.1 desktop opened successfully. Native screenshot inspection
  confirmed both new skill entries, enabled state and configured adapter labels
  in an isolated inspection profile. The normal profile independently contains
  all 20 enabled skills. Its existing permission preference was preserved.
- Live `qwen3.5:4b` loaded both skills and two references, completed seven design
  actions with seven exact per-action approvals and no unrelated actions or
  runtime errors. Both editable outputs and render reports were checked; the
  agent truthfully handed off pending visual review. Final release runtime:
  46.70 seconds.

Acceptance commands (synthetic fixtures only):

```powershell
./build.ps1 -Action test
./build.ps1 -Action clippy -CargoArguments @('--all-targets','--','-D','warnings')
<private-python> -B scripts/test_design_runner.py -v
<private-python> -B scripts/verify_design_skills.py `
  --agent target/release/vmt-agent-check.exe --output .local/design-acceptance-new
./target/release/vmt-agent-check.exe --design-live qwen3.5:4b <design-packages-root>
```

Generated fixtures, executable paths, raw source bodies and local runtime settings
remain outside Git. Local renderer/model acceptance does not certify arbitrary
document sizes, all content/layouts, or hosted CI success.

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

## V2 colour adapter increment

The optional `--colour-v2` setup flag verifies the supplied `colour-intelligence-v2.zip` against SHA-256 `20d4fad76e54bfb54643fb08c2273fb9b0ba544eb9da757a927f436ebc4ff381` and installs it alongside both existing V1 packages. Use a separate profile for evaluation; existing V1 operation names are unchanged. The new operations are `repair` (fixed 4.5:1), `gradient` (2–64), `categorical` (2–12), `simulate` (protan/deutan/tritan), `system` (light/dark), and `distance` (Euclidean OKLab). All use the reviewed V2 package and the same per-action broker. The base and advanced scripts must share a fingerprint. No generic script execution is added. The V2 document package is not yet wired in; system colour tokens are not yet consumed by compose. CI dispatch fixtures are not evidence of real-package or live-model acceptance. The existing live V1 acceptance command expects exactly two skills; use the original V1-only profile for it.

## V2 document adapter increment

`--document-v2` installs the reviewed `professional-document-design-v2.zip` (SHA-256 `a83e33b35a0ba7d30b6099fb13078f141d58e31a9439c17cfe9ec4dbe8744e28`). The separate `compose_v2` operation uses its reviewed native Word/PowerPoint composition functions. The broker supplies Windows Arial font resolution instead of the upstream Linux-only font command. V2 supports the optional design_system colour, typography and layout fields, dense paragraphs and native slide tables/continuations within the existing limits. Word currently supports white page backgrounds only; Windows supports Arial only. `system` colour output's `colour` object can be copied into the V2 design_system; no automatic composition workflow is claimed. Run render separately: it now returns bounded text-overlap and page-boundary findings, while visual review remains PENDING. No automatic repair, visual certification, or live V2 acceptance has been established. Use an isolated profile for testing. The V1 adapter remains unchanged.

## Checked V2 composition

`compose_v2` now renders its candidate before publication. Text and primary roles that fail 4.5:1 against the page background are replaced with the better black/white alternative, with changes disclosed in the result. PowerPoint candidates can be regenerated up to the approved 1–3 attempts with reduced lines per continuation slide. Word receives one structural check; no Word geometry repair is claimed. If checks fail, composition fails and the broker does not publish the candidate. The total worker budget is unchanged. A structural pass only checks extracted text geometry, not missing content, aesthetics or full accessibility. Use the separate render operation to obtain visual review PNGs. The earlier V2 paragraph's statement that composition has no automatic repair is superseded by this increment.

## Typography Intelligence (initial)

The repository includes a portable guidance package at `skills/typography-intelligence`. Import that library through the existing Skills manager or CLI, approve its fingerprint, and use `design_tool` operations `type_scale` and `measure_text`. The fixed broker worker implements these operations; the reviewed manifest is fingerprinted but never executed. The first produces suggested point sizes and spacing; the second uses Pillow and a locally approved Arial (Windows) or Liberation Sans (Linux) font for approximate wrapping geometry. No font redistribution, arbitrary font paths, OS font discovery, font pairing implementation or complex-script certification is included. Tool results are advisory and must be followed by native rendering. Existing compose_v2 accepts its supported body size and font tokens; other scale values are advisory until the layout engine supports them.

## Layout Intelligence V1

Import `skills/layout-intelligence` through the reviewed skills registry (select the parent `skills` library). `layout_grid` and `layout_check` take a single project-relative JSON input, limited to 64 KiB, fingerprinted at approval and execution. They return point-based rectangular geometry, not a rendered artifact. Grid generation supports up to 12 by 12 cells, and validation supports 64 elements with safe-margin and collision checks. No arbitrary paths or scripts are accepted. The geometry is advisory; it does not yet drive native Office shapes automatically. See the skill manifest for exact contracts.

## Composition and vector graphics V1

Import the repository `skills` library and review updated layout and new vector-graphics fingerprints. `layout_arrange` aligns edges/centres or distributes equal gaps; it returns geometry and a fresh review without modifying the source. `svg_compose` publishes a new editable SVG using a fixed XML serializer, allowlisted shapes and single-line measured text, with reported rectangular collisions. No raw markup, links, external assets or arbitrary fonts. Intentional layers may produce REVIEW_REQUIRED; visual review remains pending. PNG/PDF export and native Office layout integration are not included. CI tests cover arrangement and shape serialization; live-model acceptance remains separate.

## Vector graphics V2 scene primitives

`svg_compose` now accepts normalized polygons, internal linear/radial gradient definitions and editable multiline text with measured whitespace wrapping. It rejects overflow and unbreakable words. No new executable or dependency is required. Raw SVG markup, external URLs, arbitrary font paths and images remain unsupported. The previous single-line restriction is superseded. Geometry review is not a visual review; native rendering and PNG/PDF export remain future work.

## Vector graphics export
The same `svg_compose` operation now accepts new SVG, PNG or PDF outputs. All formats are derived from the same validated JSON and the fixed SVG serializer. PNG and PDF rendering use the existing pinned PyMuPDF dependency, not arbitrary SVG inputs or new processes. The broker verifies the resulting artifact hash and publishes without overwrite. Keep the scene JSON and SVG as editable sources. Raster export is bounded to 4096 pixels per edge and 16 million pixels total; PDF page dimensions are checked. Visual review remains PENDING. This does not establish SVG renderer fidelity for every gradient, font or effect.

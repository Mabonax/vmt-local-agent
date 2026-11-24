# Existing DOCX evaluation and response copying — 0.6.2

## What failed

The user's reported "improved" CV was an exact copy of the original. The earlier
integration could compose from a JSON contract and render Office documents, but
could not format an existing DOCX. The model attempted a UTF-8 file read and
misused rendering as editing. Review directories could be named `.docx`, making
them look like output documents. A warning was prepended to a fabricated success
summary, leaving contradictory claims visible in the UI.

The first revised live trial genuinely loaded both supplied skills and generated
a changed, content-preserving CV, but the general `design_tool` interface caused
invalid fields and missing rendering arguments. The run correctly failed its
acceptance check. A subsequent trial formatted and rendered the output, but the
test harness denied the original render because the model chose a different
review-directory name. The harness now approves any validated new review
directory for either exact fixture document; it still denies unrelated files,
shell commands, network, memory and arbitrary writes. This is a test scope
correction, not a product permission relaxation.

## Tool changes

The native model now sees small, separate function schemas:

- `colour_palette(seed, mode?, theme?)`
- `colour_contrast(foreground, background)`
- `colour_extract(input, count?)`
- `inspect_document(input)`
- `format_document(input, output, accent, preset?)`
- `render_document(input, output)`
- `compose_document(input, output)`

The legacy `design_tool` JSON interface remains supported for compatibility.
All functions share the same typed proposal, workspace checks, pinned package
verification, explicit action review and process limits. Read-only mode hides
and blocks them. A tool/proposal name must match its exact operation at dispatch.

Inspection reads actual DOCX body paragraphs, styles and tables. Formatting
patches only `word/document.xml` and `word/styles.xml`; every text and field-text
node is checked unchanged, and all other package parts are compared byte-for-byte.
The CV preset standardizes typography, spacing, heading contrast and page margins;
substantial English-heading CVs start Education on a deliberate new page. It does
not rewrite content or invent experience. Unsupported active content and external
relationships remain blocked. Existing nonstandard/complex Word layouts are not
certified by this preset.

The chosen accent must pass 4.5:1 contrast on white. The new artifact must differ
from the source and pass a hash read-back after atomic no-overwrite publication.
Rendering rejects document-like output extensions because its output is a
directory, not a finished Word file. Successful formatting counts as a file
change in the execution record.

Failed operations are tracked by exact tool arguments. An unrelated successful
read no longer clears a failed read. Actual DOCX inspection/formatting can resolve
a failed UTF-8 read of that same source. If unresolved failures remain, the model's
proposed success narrative is withheld. Recognized Word-format requests also
require a verified formatted artifact; command execution or a file copy is not
sufficient evidence.

## Local-agent evidence

The CV task ran through the same `agent::run` engine used by the desktop, using
local Ollama `qwen3.5:4b`, both registered user-supplied skills and an isolated copy
of the actual source. Codex did not author the final document directly. The local
model selected and audited teal `#007B7F` (5.0729:1 on white), called the formatter,
and rendered both original and result. The exact final run took 36.94 seconds:

| Check | Result |
| --- | --- |
| Both skills loaded | PASS |
| Actual colour palette and contrast operations | PASS |
| Palette-derived accent applied to document | PASS |
| Exact source text/fields and opaque package parts preserved | PASS |
| Original file and its isolated input copy unchanged | PASS |
| Formatted output is a real, changed DOCX | PASS |
| Source render | 3 pages |
| Formatted render | 2 pages |
| Explicit reviewed design actions | 6 |
| Failed tools, unrelated actions, runtime errors | 0 |

Every original and final page image was inspected. The final has readable body
text, consistent headings and a deliberate section break, without clipped text
or overlap. The local model correctly leaves visual review to a human: this
increment does not add an image-inspection tool. The exported user deliverable is
an unchanged copy of the model-produced artifact. Raw documents, responses,
hash receipts and trial reports remain in ignored `.local` storage, not Git.

Use the explicit local evaluation helper with an operator-selected DOCX:

```powershell
./target/release/vmt-agent-check.exe --document-task qwen3.5:4b C:/path/to/input.docx
```

This helper approves only the bounded fixture actions above; it never changes
normal-profile permission settings. One successful CV run does not establish
general Word editing quality or reliable execution on every local model.

## Copying responses

The native clipboard feature was omitted from the earlier eframe build, causing
the framework to use its internal clipboard. The build now enables `clipboard`.
Full-response copy uses an explicit native write with bounded retries and visible
Copied/error feedback. No existing clipboard content is read by this handler.
Response bodies and the full-text preview support read-only keyboard selection;
code blocks retain selection and Copy code. Copy controls appear at the top and
bottom of assistant responses.

The packaged Windows release was tested with a synthetic response containing
paragraphs and a fenced code block. Copy response displayed Copied, and Ctrl+V
in the composer reproduced the full response including the code and final line.
Ctrl+A selected the actual read-only response text. No test message was sent and
no existing clipboard content was inspected. The desktop shortcut targets 0.6.2.

The final local checks passed 49 Rust tests (46 library and three broker
integration cases), eight Python adapter tests, strict Clippy, formatting and
diff checks. Hosted CI is a separate result from these local checks.

Regression checks cover false success suppression, unrelated-success recovery,
native function schemas/readonly filtering, unchanged content/opaque parts,
bad contrast, output-directory confusion and existing permission boundaries.

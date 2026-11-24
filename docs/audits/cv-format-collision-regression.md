# Existing CV formatting: output collision regression

Date: 2026-11-24. Follow-up to the local creative engine acceptance.

## Reported failure and cause

An ordinary existing-DOCX formatting task made two identical formatted copies,
rendered both, and then ended incomplete with 21 failed attempts. Activity showed
the first requested output already existed. The validator combined that condition
with a missing-parent error. The model tried unnecessary PowerShell directory
commands, including unsupported shell syntax, and retried existing output names.
Completion recovery matched exact arguments, so a successful new output could not
resolve the earlier rejected output name.

## Changes

- Output validation distinguishes a collision from a missing parent and suggests
  an unused sibling filename. It creates nothing and never approves execution.
- Formatting guidance shared by native and JSON protocols directs the model to
  dedicated tools, existing directories, one changed copy, then rendering. It
  discourages coding workflows, shell workarounds and duplicate formatting.
- Failure reconciliation distinguishes validation from dispatched execution.
  A successful equivalent formatting/render operation can replace a validation
  failure with a different output name. Input and other parameters must match.
  Execution failures, unrelated inputs and failed commands remain unresolved.
- A third request for the same twice-failed action stops the turn, including when
  other actions intervene. Activity and total failure counts remain truthful.
- `--document-collision-task MODEL SOURCE.docx` creates an isolated project with a
  pre-existing output and verifies its preservation. Normal review still applies;
  the fixture approves only the declared document/colour operations.

## Evidence

`build.ps1 -Action test` passed all 48 library tests and three integration tests.
Strict all-target Clippy, formatting checks and the locked release desktop build
also passed. The rebuilt normal-profile app was restarted and observed Ready;
its selected local model remained `qwen3.5:4b` with 16,384 context tokens and 2,048
output tokens. No permission or inference-provider defaults were changed.

The final real `qwen3.5:4b` run at 16,384 context tokens passed in 64.55 seconds:
8 completed actions, one changed document, zero commands, zero web requests, zero
denials and one safely rejected filename collision. It loaded both skills,
inspected the source, audited and applied a palette accent, recovered to the
suggested unused filename, and rendered the source and formatted copy. Six exact
design proposals were approved by the isolated acceptance fixture. The source and
pre-existing output stayed byte-identical; the new document preserved its content.

The first run failed a harness check for a literal review-directory name although
both correct artifacts had been rendered. The harness now verifies each actual
broker render result against the document SHA-256, its on-disk receipt and all
reported page images. It requires **both** original and formatted renders and
reports whether the requested directory names were used. The final model still
chose different review-directory names; that naming mismatch remains in its report.

Both final formatted pages and both pages of the previously created desktop copy
were visually inspected by Codex: readable text, consistent headings, no observed
clipping or overlap. The existing desktop copy also retained all text and package
parts; only document/style XML changed. This external inspection does not give the
local model an image-viewing capability or prove every future formatting request.

Private reports, source copies, responses and renders stay under `.local`; no CV
content is included in this audit. Existing user files were not overwritten.

---
name: vector-graphics
description: Generate editable SVG scenes from bounded native rectangles, ellipses and single-line text with explicit layers and colours.
metadata:
  version: "1.0"
---
# Vector graphics
Use layout-intelligence and typography-intelligence to plan geometry and colour-intelligence for palette selection. Write reviewed JSON then call design_tool with operation svg_compose, input and new output ending in .svg.

Example:
```json
{"width":600,"height":300,"background":"#FFFFFF","elements":[{"id":"panel","kind":"rect","x":20,"y":20,"w":560,"h":260,"fill":"#EAF2F3","radius":12},{"id":"heading","kind":"text","x":40,"y":40,"w":500,"h":60,"fill":"#172B45","text":"Design with purpose","size":24,"weight":"normal"}]}
```

Coordinates and sizes are points. Rectangles support radius; ellipses use their bounding box. Text uses approved local Arial or Liberation Sans, size 8–72, normal or bold, single line. Object order determines stacking. Overlaps are reported, including intentional layers. No arbitrary XML, scripts, hyperlinks, images, external references or user-supplied font paths. Review the rendered appearance in an SVG viewer; geometric checks are not aesthetic certification.

## V2 scene extensions
Polygons use `kind: polygon` and `points` containing 3–64 pairs of normalized coordinates (0–1) relative to their declared box. Shape or text fill accepts a HEX string or a gradient object: `{"kind":"linear","direction":"diagonal","stops":[{"offset":0,"colour":"#123456"},{"offset":1,"colour":"#FFFFFF"}]}`. Linear directions: horizontal, vertical, diagonal. Radial gradients omit direction. Stops must be ordered, 2–8, offsets 0–1. Text can contain newline characters and wraps on whitespace within the declared box; optional line_height 1–2. At most 24 lines and 1600 characters; oversized words and excess height are rejected. Native text spans remain editable. This supersedes the single-line restriction. External URLs and raw markup remain unsupported.

## Export
Use the same reviewed JSON scene with `svg_compose` and a new output ending in `.svg`, `.png` or `.pdf`. Generate SVG first to retain an editable source. PNG/PDF are rendered only from the worker's freshly serialized scene using PyMuPDF. PNG targets 144 DPI, reduced for large canvases, with 4096-pixel edge and 16-million-pixel limits. PDF preserves the canvas size; text may be outlined and effects may be flattened. Export does not certify visual fidelity, print colour management or accessibility. Arbitrary external SVG input is not supported.

## Reviewed image assets
Pass `assets` (up to four project-relative PNG/JPEG/WebP paths) in the svg_compose request, not the scene. Image elements use `kind: image`, `asset: 0` (zero-based index), `fit: contain` or `cover`, and id/x/y/w/h; omit fill. All objects accept opacity 0–1. The broker hashes each asset before approval and rechecks it before copying into the private worker directory. The worker re-encodes images as metadata-free PNG, limits each to 1600 pixels on its longest edge, and embeds only internally generated data URLs. No source paths or arbitrary URLs in scene objects. Visual fidelity of image rendering remains subject to review.

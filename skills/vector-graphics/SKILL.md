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

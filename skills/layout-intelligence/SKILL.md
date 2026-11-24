---
name: layout-intelligence
description: Construct consistent page and slide grids, allocate rectangular content space and detect collisions or safe-area violations before rendering.
metadata:
  version: "1.0"
---
# Layout Intelligence

Use points with a top-left origin. Decide canvas size, safe margins, columns and gutters according to the medium. Prefer consistent alignment, meaningful grouping, visual hierarchy and white space. Avoid decorative grids that compete with content.

## Tools

Write a project-relative JSON input using the normal reviewed file tool. Invoke design_tool with operation layout_grid and input path.

Example input:
```json
{"width":960,"height":540,"margin":30,"gutter":12,"columns":3,"rows":2}
```

Grid returns rectangular cells with row, column, x, y, w, h. Combine adjacent cells mentally or in a subsequent approved layout plan. All dimensions are points.

To check a composition, use layout_check with an input like:
```json
{"width":960,"height":540,"margin":20,"elements":[{"id":"title","x":20,"y":20,"w":920,"h":80},{"id":"body","x":20,"y":120,"w":920,"h":380}]}
```

Correct unexpected overlaps and safe-area violations before generating artwork. Use typography-intelligence measure_text for representative text fit. Split dense content rather than reducing type indefinitely. Intentional overlays require separate explicit review. The validator checks rectangles, not optical balance, actual text fit, native rendering or aesthetic quality. Do not claim these are certified. Render final output and inspect it separately.

---
name: creative-director
description: Coordinate a bounded creative planning workflow, rank scene alternatives, preserve supplied content and report deterministic design repairs.
metadata:
  version: "1.0"
---
# Creative Director
Establish the user's purpose, audience, medium, factual content and approved brand colours. Use colour-intelligence, typography-intelligence, layout-intelligence and image-intelligence as needed. Load creative-templates for the supported brief schema. Write the brief JSON through the reviewed file tool, then call design_tool creative_plan with its project-relative path.
The planner generates standard, compact and expanded typography variants. It applies at most two overflow repair passes per variant, preserves content, and ranks by geometric/text findings. It does not judge aesthetics. Review reported colour and size changes. If NEEDS_REVISION, correct the brief or layout; never silently remove text. READY_FOR_RENDER_REVIEW means only the deterministic checks passed.
Save the selected scene using the reviewed file tool, run svg_compose with explicit asset paths if needed, and inspect output visually before acceptance. Do not bypass approval, execute arbitrary scripts or claim a visual pass without evidence. This is a planning workflow, not persistent autonomous execution.

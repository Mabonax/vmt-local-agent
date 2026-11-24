---
name: image-intelligence
description: Inspect local raster images and prepare them for design without distortion, preserving source files and reporting enlargement.
metadata:
  version: "1.0"
---
# Image Intelligence
Load this skill before calling design_tool image_inspect or image_prepare. Use project-relative PNG, JPEG or WebP files only. Animated files are unsupported. Inspect source dimensions, aspect ratio, transparency and ICC presence before choosing a target. Never infer visual content from metadata alone.

Use image_prepare with input, output, width and height in pixels, and mode contain or cover. Contain fits the entire image with transparent padding (white for JPEG). Cover uses a centered crop. Both preserve aspect ratio. Report enlargement; it does not recover detail. EXIF orientation is applied and output metadata removed. ICC conversion is not performed. Keep original assets. Use separate visual review before delivery. Do not claim image embedding in vector scenes is supported yet.

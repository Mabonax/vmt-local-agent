---
name: image-intelligence
description: Inspect local raster images and prepare them for design without distortion, preserving source files and reporting enlargement.
metadata:
  version: "1.0"
---
# Image Intelligence
Load this skill before calling design_tool image_inspect or image_prepare. Use project-relative PNG, JPEG or WebP files only. Animated files are unsupported. Inspect source dimensions, aspect ratio, transparency and ICC presence before choosing a target. Never infer visual content from metadata alone.

Use image_prepare with input, output, width and height in pixels, and mode contain or cover. Contain fits the entire image with transparent padding (white for JPEG). Cover uses a centered crop. Both preserve aspect ratio. Report enlargement; it does not recover detail. EXIF orientation is applied and output metadata removed. ICC conversion is not performed. Keep original assets. Use separate visual review before delivery. Do not claim image embedding in vector scenes is supported yet.

## Image adjustments
Use `image_adjust` with input, output and mode. Brightness, contrast, saturation and sharpness take amount 0–200 percent (100 neutral). Grayscale and circle take no amount; circle crops to a centered square then masks it. Rounded uses amount 0–50 percent of the shorter edge (default 20). Masks require PNG or WebP. Existing alpha is preserved/multiplied; original assets remain unchanged. Tonal metrics are simple visible-pixel statistics, not a judgement of quality. No subject recognition or ICC conversion. Review the result visually.

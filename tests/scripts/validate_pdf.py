#!/usr/bin/env python3
"""Validate PDF appearance using PyMuPDF (fitz) and Pillow.

Exit codes:
 - 0 success
 - 1 validation failure
 - 77 skipped because required python modules are not available
"""
import argparse
import sys

try:
    import fitz  # PyMuPDF
    from PIL import Image
except Exception as e:
    print("Missing Python modules (PyMuPDF/Pillow). To enable image validation install: pip install pymupdf Pillow")
    sys.exit(77)

import io

MM_TO_PT = 72.0 / 25.4

parser = argparse.ArgumentParser()
parser.add_argument("--pdf", required=True)
parser.add_argument("--expect-image", action="store_true", help="Fail if no images found on page")
parser.add_argument("--positions", action="append", help="Expected image area(s) as x_mm,y_mm,w_mm,h_mm (can repeat)")
parser.add_argument("--page", type=int, default=1)
parser.add_argument("--dpi", type=int, default=150)
parser.add_argument("--ref-source", help="Reference source image to use for pixel comparisons (path)")
parser.add_argument("--threshold", type=float, default=0.08, help="Normalized RMS threshold for pixel comparison (0..1)")
parser.add_argument("--save-diff", help="Path to directory where difference images will be saved on failure")
args = parser.parse_args()

pdf_path = args.pdf
p = fitz.open(pdf_path)
page_no = max(0, args.page - 1)
if page_no >= len(p):
    print(f"Page {args.page} out of range (document has {len(p)})")
    sys.exit(1)
page = p[page_no]

# If expect-image, ensure page.get_images returns at least one
images = page.get_images()
if args.expect_image and not images:
    print(f"No images found on page {args.page}")
    sys.exit(1)

# Render page to pixmap
mat = fitz.Matrix(args.dpi / 72.0, args.dpi / 72.0)
pix = page.get_pixmap(matrix=mat, alpha=False)
img = Image.frombytes("RGB", [pix.width, pix.height], pix.samples)
width_px, height_px = img.size

# Helper: convert mm rect to pixel bbox
page_rect = page.rect  # in points
page_height_pt = page_rect.height

def mm_to_px(x_mm, y_mm, dpi=args.dpi):
    # convert mm to points, then convert to pixels
    pt_x = x_mm * MM_TO_PT
    pt_y = y_mm * MM_TO_PT
    px_x = pt_x * (dpi / 72.0)
    # PyMuPDF image has origin top-left; mm positions are given with origin bottom-left in our tests
    px_y = (page_height_pt - pt_y) * (dpi / 72.0)
    return int(px_x), int(px_y)

# Validate provided positions
if args.positions:
    for idx, pos in enumerate(args.positions):
        try:
            x_mm, y_mm, w_mm, h_mm = [float(v) for v in pos.split(",")]
        except Exception:
            print(f"Invalid position format: {pos}")
            sys.exit(1)
        cx, cy = mm_to_px(x_mm, y_mm)
        w_px = int(w_mm * MM_TO_PT * (args.dpi / 72.0))
        h_px = int(h_mm * MM_TO_PT * (args.dpi / 72.0))

        # crop a small box centered on the coordinates
        left = max(0, cx - w_px // 2)
        upper = max(0, cy - h_px // 2)
        right = min(width_px, cx + w_px // 2)
        lower = min(height_px, cy + h_px // 2)

        if left >= right or upper >= lower:
            print(f"Computed empty crop for position {pos}")
            sys.exit(1)

        crop = img.crop((left, upper, right, lower)).convert("RGB")

        # Simple presence check: ensure non-white pixels exist
        gray = crop.convert("L")
        bbox = gray.getbbox()
        if bbox is None:
            print(f"No non-white pixels found in expected area {pos} (crop {left},{upper}-{right},{lower})")
            sys.exit(1)

        print(f"Found non-white content for position {pos} in crop {left},{upper}-{right},{lower}")

        # If reference source provided, perform pixel-wise comparison
        if args.ref_source:
            try:
                ref = Image.open(args.ref_source).convert("RGB")
            except Exception as e:
                print(f"Cannot open reference source image {args.ref_source}: {e}")
                sys.exit(1)

            # Resize reference to crop size for comparison (best-effort)
            ref_resized = ref.resize((right - left, lower - upper), Image.LANCZOS)

            # compute RMS error
            from PIL import ImageChops
            diff = ImageChops.difference(crop, ref_resized)
            # compute mean squared error
            hist = diff.histogram()
            # histogram is for each channel concatenated; compute sum of squares
            sq = 0
            for i, val in enumerate(hist):
                channel_val = i % 256
                sq += (channel_val * channel_val) * val
            mse = sq / float((right - left) * (lower - upper) * 3)
            import math
            rmse = math.sqrt(mse) / 255.0

            print(f"Position {idx}: normalized RMSE = {rmse:.6f} (threshold {args.threshold})")
            if rmse > args.threshold:
                print(f"Pixel difference too large for position {pos} (rmse {rmse} > {args.threshold})")
                # save debugging images if requested
                if args.save_diff:
                    import os
                    os.makedirs(args.save_diff, exist_ok=True)
                    crop.save(os.path.join(args.save_diff, f"crop_{idx}.png"))
                    ref_resized.save(os.path.join(args.save_diff, f"ref_{idx}.png"))
                    diff.save(os.path.join(args.save_diff, f"diff_{idx}.png"))
                    print(f"Saved debug images to {args.save_diff}")
                sys.exit(1)
            else:
                print(f"Pixel comparison OK for position {pos}")

print("Validation OK")
sys.exit(0)

# Lessons Learned

## 2026-03-04

- Mistake pattern: Using `object-fit: cover` with forced aspect ratios on unknown asset dimensions can silently crop key product screenshots.
- Preventive rule: For product demo images, default to `object-fit: contain` and preserve native aspect ratio unless explicit art direction requires cropping.
- Preventive rule: After major visual revamps, run a browser-level screenshot check before sign-off.

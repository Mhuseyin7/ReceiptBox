# OCR

The native backend checks for a local `tesseract` runtime. It reports `available` only when the binary executes successfully; otherwise it reports the missing dependency clearly. This prevents fabricated OCR success.

Production packaging remains pending: bundle a licensed Tesseract binary with `eng` and `tur` language data per target platform, invoke it with fixed argument arrays, capture per-page failures, and test release packages. No hosted OCR service is used.

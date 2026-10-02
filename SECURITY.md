# Security

ReceiptBox treats imported documents as untrusted input. Current safeguards include extension plus magic-byte validation, a 25 MiB import cap, generated storage names, SHA-256 duplicate detection, parameterized SQLite statements, and a command interface that does not accept shell strings.

The native Tesseract availability check uses a fixed executable and argument vector. No OCR text is rendered as HTML.

Known work before a security release: PDF parser sandboxing, image pixel/decompression limits, symlink containment checks, encrypted-library threat model, signed release artifacts, external security review, and secure trash retention.

# ReceiptBox architecture

ReceiptBox is intentionally local-first. The React frontend runs inside a Tauri WebView and invokes a deliberately small native command surface. The Rust process owns document paths and SQLite; frontend code never constructs storage paths.

`import_document` validates the extension and file signature, applies the size limit, computes SHA-256, detects byte-identical documents, and copies the original to an application-generated ID under the library. SQLite stores the original path and metadata. FTS5 is maintained through database triggers, so edits remain searchable.

The database currently has documents, a derived FTS index, and audit events. SQLite foreign keys are enabled at startup. Backups copy the database and originals into a new directory together with a versioned JSON manifest. A restore command is deliberately not exposed until overwrite confirmation and archive-integrity checks are implemented.

OCR is isolated behind a runtime capability check. The app never declares it available unless Tesseract itself responds successfully. It does not send documents over the network.

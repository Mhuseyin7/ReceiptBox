use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};
use tauri::{Manager, State};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
enum ReceiptError {
    #[error("unsupported document type")]
    UnsupportedType,
    #[error("document exceeds the configured 25 MiB size limit")]
    TooLarge,
    #[error("document signature does not match its supported type")]
    BadSignature,
    #[error("file is not a regular file")]
    NotFile,
    #[error("{0}")]
    Other(String),
}
impl Serialize for ReceiptError {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        s.serialize_str(&self.to_string())
    }
}
type Result<T> = std::result::Result<T, ReceiptError>;

pub struct AppState {
    db: Mutex<Connection>,
    library: PathBuf,
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: String,
    pub name: String,
    pub merchant: String,
    pub date: String,
    pub total: String,
    pub currency: String,
    pub category: String,
    pub status: String,
    pub text: String,
    pub hash: String,
    pub added: String,
    pub original_path: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub document: Option<Document>,
    pub duplicate_of: Option<String>,
}

fn app_error<E: std::fmt::Display>(e: E) -> ReceiptError {
    ReceiptError::Other(e.to_string())
}
fn initialize(conn: &Connection) -> std::result::Result<(), rusqlite::Error> {
    conn.execute_batch("PRAGMA foreign_keys=ON;
 CREATE TABLE IF NOT EXISTS documents (id TEXT PRIMARY KEY, name TEXT NOT NULL, merchant TEXT NOT NULL DEFAULT 'Unrecognized merchant', purchase_date TEXT NOT NULL DEFAULT 'Unknown date', total TEXT NOT NULL DEFAULT '—', currency TEXT NOT NULL DEFAULT '—', category TEXT NOT NULL DEFAULT 'Uncategorized', status TEXT NOT NULL DEFAULT 'Needs review', ocr_text TEXT NOT NULL DEFAULT '', content_hash TEXT NOT NULL UNIQUE, original_path TEXT NOT NULL, imported_at TEXT NOT NULL);
 CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(id UNINDEXED, name, merchant, category, ocr_text);
 DROP TRIGGER IF EXISTS documents_ai;
 DROP TRIGGER IF EXISTS documents_au;
 CREATE TRIGGER documents_ai AFTER INSERT ON documents BEGIN INSERT INTO documents_fts(rowid,id,name,merchant,category,ocr_text) VALUES(new.rowid,new.id,new.name,new.merchant,new.category,new.ocr_text); END;
 CREATE TRIGGER documents_au AFTER UPDATE ON documents BEGIN DELETE FROM documents_fts WHERE rowid=old.rowid; INSERT INTO documents_fts(rowid,id,name,merchant,category,ocr_text) VALUES(new.rowid,new.id,new.name,new.merchant,new.category,new.ocr_text); END;
 CREATE TABLE IF NOT EXISTS audit_events (id TEXT PRIMARY KEY, document_id TEXT, event TEXT NOT NULL, created_at TEXT NOT NULL);")
}
fn magic(path: &Path) -> Result<()> {
    let data = fs::read(path).map_err(app_error)?;
    if data.len() > 25 * 1024 * 1024 {
        return Err(ReceiptError::TooLarge);
    };
    if data.len() < 4 {
        return Err(ReceiptError::BadSignature);
    };
    let ext = path
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let valid = match ext.as_str() {
        "pdf" => data.starts_with(b"%PDF-"),
        "jpg" | "jpeg" => data.starts_with(&[0xff, 0xd8, 0xff]),
        "png" => data.starts_with(b"\x89PNG\r\n\x1a\n"),
        "webp" => data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP"),
        "tif" | "tiff" => data.starts_with(b"II*\0") || data.starts_with(b"MM\0*"),
        _ => return Err(ReceiptError::UnsupportedType),
    };
    if valid {
        Ok(())
    } else {
        Err(ReceiptError::BadSignature)
    }
}
fn checksum(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(app_error)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}
fn to_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?,
        name: row.get(1)?,
        merchant: row.get(2)?,
        date: row.get(3)?,
        total: row.get(4)?,
        currency: row.get(5)?,
        category: row.get(6)?,
        status: row.get(7)?,
        text: row.get(8)?,
        hash: row.get::<_, String>(9)?[..8].to_owned(),
        added: row.get(10)?,
        original_path: row.get(11)?,
    })
}
fn query_docs(conn: &Connection, sql: &str, term: Option<&str>) -> Result<Vec<Document>> {
    let mut st = conn.prepare(sql).map_err(app_error)?;
    let mut out = Vec::new();
    if let Some(t) = term {
        let rows = st.query_map([t], to_document).map_err(app_error)?;
        for r in rows {
            out.push(r.map_err(app_error)?)
        }
    } else {
        let rows = st.query_map([], to_document).map_err(app_error)?;
        for r in rows {
            out.push(r.map_err(app_error)?)
        }
    };
    Ok(out)
}
const DOC_COLUMNS:&str="id,name,merchant,purchase_date,total,currency,category,status,ocr_text,content_hash,imported_at,original_path";
fn build_fts_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .map(|term| {
            term.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|term| !term.is_empty())
        .filter(|term| {
            !matches!(
                term.to_ascii_uppercase().as_str(),
                "AND" | "OR" | "NOT" | "NEAR"
            )
        })
        .collect();
    (!terms.is_empty()).then(|| {
        terms
            .iter()
            .map(|term| format!("{term}*"))
            .collect::<Vec<_>>()
            .join(" AND ")
    })
}

#[tauri::command]
fn list_documents(state: State<'_, AppState>) -> Result<Vec<Document>> {
    let conn = state.db.lock().map_err(app_error)?;
    query_docs(
        &conn,
        &format!("SELECT {DOC_COLUMNS} FROM documents ORDER BY imported_at DESC"),
        None,
    )
}
#[tauri::command]
fn search_documents(query: String, state: State<'_, AppState>) -> Result<Vec<Document>> {
    let q = query.trim();
    if q.is_empty() {
        return list_documents(state);
    };
    let Some(fts_query) = build_fts_query(q) else {
        return Ok(Vec::new());
    };
    let conn = state.db.lock().map_err(app_error)?;
    query_docs(
        &conn,
        &format!("SELECT d.{DOC_COLUMNS} FROM documents d JOIN documents_fts f ON f.id=d.id WHERE documents_fts MATCH ? ORDER BY rank"),
        Some(&fts_query),
    )
}
#[tauri::command]
fn import_document(source_path: String, state: State<'_, AppState>) -> Result<ImportOutcome> {
    let source = PathBuf::from(source_path);
    if !source.is_file() {
        return Err(ReceiptError::NotFile);
    };
    magic(&source)?;
    let hash = checksum(&source)?;
    let conn = state.db.lock().map_err(app_error)?;
    if let Some(existing) = conn
        .query_row(
            "SELECT id FROM documents WHERE content_hash=?",
            [&hash],
            |r| r.get(0),
        )
        .optional()
        .map_err(app_error)?
    {
        return Ok(ImportOutcome {
            document: None,
            duplicate_of: Some(existing),
        });
    };
    let id = Uuid::new_v4().to_string();
    let ext = source.extension().and_then(|x| x.to_str()).unwrap_or("bin");
    let dest = state.library.join("originals").join(format!("{id}.{ext}"));
    fs::copy(&source, &dest).map_err(app_error)?;
    let now = Utc::now().to_rfc3339();
    let name = source
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or("document")
        .to_string();
    conn.execute(
        "INSERT INTO documents(id,name,content_hash,original_path,imported_at) VALUES(?,?,?,?,?)",
        params![id, name, hash, dest.to_string_lossy(), now],
    )
    .map_err(app_error)?;
    conn.execute(
        "INSERT INTO audit_events(id,document_id,event,created_at) VALUES(?,?,?,?)",
        params![
            Uuid::new_v4().to_string(),
            id,
            "imported",
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(app_error)?;
    let d = conn
        .query_row(
            &format!("SELECT {DOC_COLUMNS} FROM documents WHERE id=?"),
            [id],
            to_document,
        )
        .map_err(app_error)?;
    Ok(ImportOutcome {
        document: Some(d),
        duplicate_of: None,
    })
}
#[tauri::command]
fn save_document(document: Document, state: State<'_, AppState>) -> Result<Document> {
    let conn = state.db.lock().map_err(app_error)?;
    conn.execute("UPDATE documents SET merchant=?,purchase_date=?,total=?,currency=?,category=?,status=?,ocr_text=? WHERE id=?",params![document.merchant,document.date,document.total,document.currency,document.category,document.status,document.text,document.id]).map_err(app_error)?;
    conn.query_row(
        &format!("SELECT {DOC_COLUMNS} FROM documents WHERE id=?"),
        [document.id],
        to_document,
    )
    .map_err(app_error)
}
#[tauri::command]
fn export_csv(state: State<'_, AppState>) -> Result<String> {
    let docs = list_documents(state)?;
    let mut result = String::from("Date,Merchant,Category,Currency,Total,Document type\n");
    for d in docs {
        let esc = |v: String| csv_escape(&v);
        result.push_str(&format!(
            "{},{},{},{},{},\"Receipt\"\n",
            esc(d.date),
            esc(d.merchant),
            esc(d.category),
            esc(d.currency),
            esc(d.total)
        ));
    }
    Ok(result)
}
fn csv_escape(value: &str) -> String {
    let protected = if matches!(value.chars().next(), Some('=' | '+' | '-' | '@')) {
        format!("'{value}")
    } else {
        value.to_owned()
    };
    format!("\"{}\"", protected.replace('"', "\"\""))
}
#[tauri::command]
fn create_backup(destination: String, state: State<'_, AppState>) -> Result<()> {
    let destination = PathBuf::from(destination);
    if destination.exists() {
        return Err(ReceiptError::Other("backup target already exists".into()));
    };
    let conn = state.db.lock().map_err(app_error)?;
    conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
        .map_err(app_error)?;
    drop(conn);
    fs::create_dir_all(&destination).map_err(app_error)?;
    fs::copy(
        state.library.join("receiptbox.sqlite3"),
        destination.join("receiptbox.sqlite3"),
    )
    .map_err(app_error)?;
    copy_dir(
        &state.library.join("originals"),
        &destination.join("originals"),
    )?;
    let manifest = serde_json::json!({"format":"receiptbox-backup","version":1,"createdAt":Utc::now().to_rfc3339()});
    fs::write(
        destination.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(app_error)?,
    )
    .map_err(app_error)
}
fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).map_err(app_error)?;
    for item in fs::read_dir(from).map_err(app_error)? {
        let item = item.map_err(app_error)?;
        if item.file_type().map_err(app_error)?.is_file() {
            fs::copy(item.path(), to.join(item.file_name())).map_err(app_error)?;
        }
    }
    Ok(())
}
#[tauri::command]
fn ocr_runtime_status() -> String {
    match Command::new("tesseract").arg("--version").output() {
        Ok(o) if o.status.success() => "available".into(),
        _ => {
            "unavailable: install or bundle Tesseract with English and Turkish language data".into()
        }
    }
}
pub fn run() {
    tracing_subscriber::fmt().with_env_filter("info").init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let base = app.path().app_data_dir().map_err(app_error)?;
            let library = base.join("ReceiptBoxLibrary");
            fs::create_dir_all(library.join("originals")).map_err(app_error)?;
            let db = Connection::open(library.join("receiptbox.sqlite3")).map_err(app_error)?;
            initialize(&db).map_err(app_error)?;
            app.manage(AppState {
                db: Mutex::new(db),
                library,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_documents,
            search_documents,
            import_document,
            save_document,
            export_csv,
            create_backup,
            ocr_runtime_status
        ])
        .run(tauri::generate_context!())
        .expect("ReceiptBox failed to start")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn rejects_disguised_file() {
        let d = tempdir().unwrap();
        let p = d.path().join("bad.pdf");
        fs::write(&p, b"not a pdf").unwrap();
        assert!(matches!(magic(&p), Err(ReceiptError::BadSignature)))
    }
    #[test]
    fn schema_supports_fts() {
        let c = Connection::open_in_memory().unwrap();
        initialize(&c).unwrap();
        c.execute("INSERT INTO documents(id,name,content_hash,original_path,imported_at) VALUES('1','market.pdf','hash','x','now')",[]).unwrap();
        let count: i64 = c
            .query_row(
                "SELECT count(*) FROM documents_fts WHERE documents_fts MATCH 'market*'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1)
    }
    #[test]
    fn updating_a_document_refreshes_its_search_index() {
        let c = Connection::open_in_memory().unwrap();
        initialize(&c).unwrap();
        c.execute("INSERT INTO documents(id,name,merchant,content_hash,original_path,imported_at) VALUES('1','receipt.pdf','Old Shop','hash','x','now')",[]).unwrap();
        c.execute("UPDATE documents SET merchant='New Shop' WHERE id='1'", [])
            .unwrap();
        let old_count: i64 = c
            .query_row(
                "SELECT count(*) FROM documents_fts WHERE documents_fts MATCH 'Old*'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let new_count: i64 = c
            .query_row(
                "SELECT count(*) FROM documents_fts WHERE documents_fts MATCH 'New*'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!((old_count, new_count), (0, 1));
    }
    #[test]
    fn csv_cells_cannot_be_formulas() {
        assert_eq!(csv_escape("=1+1"), "\"'=1+1\"");
        assert_eq!(csv_escape("merchant"), "\"merchant\"");
    }
    #[test]
    fn fts_queries_strip_operators() {
        assert_eq!(
            build_fts_query("market + receipt"),
            Some("market* AND receipt*".into())
        );
        assert_eq!(build_fts_query("\" OR *"), None);
    }
}

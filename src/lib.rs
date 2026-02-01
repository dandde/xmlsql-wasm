use serde::{Deserialize, Serialize};
use sqlite_wasm_rs::{
    sqlite3, sqlite3_close, sqlite3_errmsg, sqlite3_exec, sqlite3_free, sqlite3_open_v2, SQLITE_OK,
    SQLITE_OPEN_CREATE, SQLITE_OPEN_MEMORY, SQLITE_OPEN_READWRITE,
};
use sqlite_wasm_rs::{
    sqlite3_bind_int64, sqlite3_bind_text, sqlite3_column_count, sqlite3_column_name,
    sqlite3_column_text, sqlite3_column_type, sqlite3_finalize, sqlite3_last_insert_rowid,
    sqlite3_prepare_v2, sqlite3_step,
};
use sqlite_wasm_rs::{
    sqlite3_deserialize, sqlite3_malloc, sqlite3_serialize, SQLITE_BLOB,
    SQLITE_DESERIALIZE_FREEONCLOSE, SQLITE_DESERIALIZE_RESIZEABLE, SQLITE_DONE, SQLITE_FLOAT,
    SQLITE_INTEGER, SQLITE_NULL, SQLITE_ROW, SQLITE_TEXT,
};
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;
use wasm_bindgen::prelude::*;

mod parser;
mod selector;

use parser::{parse_html_to_nodes, parse_xml_to_nodes};
use selector::css_to_sql;

// Use wee_alloc as the global allocator for smaller WASM binary
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeData {
    pub id: i64,
    pub tag_name: String,
    pub text_content: Option<String>,
    pub attributes: HashMap<String, String>,
    pub parent_id: Option<i64>,
    pub depth: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[wasm_bindgen]
pub struct XmlSqlDb {
    db: *mut sqlite3,
}

unsafe impl Send for XmlSqlDb {}

#[wasm_bindgen]
impl XmlSqlDb {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<XmlSqlDb, JsValue> {
        console_log!("Initializing XmlSqlDb with sqlite-wasm-rs...");

        let mut db = ptr::null_mut();
        // Use standard in-memory database
        let c_filename = CString::new(":memory:").unwrap();

        let flags = SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_MEMORY;

        let ret = unsafe { sqlite3_open_v2(c_filename.as_ptr(), &mut db, flags, ptr::null()) };

        if ret != SQLITE_OK {
            return Err(JsValue::from_str("Failed to open valid in-memory database"));
        }

        // Init schema
        if let Err(e) = init_schema_ffi(db) {
            unsafe { sqlite3_close(db) };
            return Err(JsValue::from_str(&format!(
                "Failed to initialize schema: {}",
                e
            )));
        }

        console_log!("Database initialized successfully");

        Ok(XmlSqlDb { db })
    }

    #[wasm_bindgen]
    pub fn load_xml(&mut self, content: &str, document_name: &str) -> Result<u64, JsValue> {
        console_log!("Loading XML document: {}", document_name);
        let nodes = parse_xml_to_nodes(content)
            .map_err(|e| JsValue::from_str(&format!("XML parsing failed: {}", e)))?;
        self.insert_document(document_name, &nodes)
            .map_err(|e| JsValue::from_str(&format!("Database insertion failed: {}", e)))
    }

    #[wasm_bindgen]
    pub fn load_html(&mut self, content: &str, document_name: &str) -> Result<u64, JsValue> {
        console_log!("Loading HTML document: {}", document_name);
        let nodes = parse_html_to_nodes(content)
            .map_err(|e| JsValue::from_str(&format!("HTML parsing failed: {}", e)))?;
        self.insert_document(document_name, &nodes)
            .map_err(|e| JsValue::from_str(&format!("Database insertion failed: {}", e)))
    }

    #[wasm_bindgen]
    pub fn query_selector(&self, selector: &str) -> Result<JsValue, JsValue> {
        console_log!("Executing CSS selector: {}", selector);
        let sql = css_to_sql(selector)
            .map_err(|e| JsValue::from_str(&format!("Selector parsing failed: {}", e)))?;
        console_log!("Generated SQL: {}", sql);
        self.execute_sql(&sql)
    }

    #[wasm_bindgen]
    pub fn execute_sql(&self, sql: &str) -> Result<JsValue, JsValue> {
        console_log!("Executing SQL: {}", sql);

        let mut stmt = ptr::null_mut();
        let c_sql = CString::new(sql).map_err(|_| JsValue::from_str("Invalid SQL string"))?;

        let ret =
            unsafe { sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };

        if ret != SQLITE_OK {
            let err_msg = unsafe {
                let c_str = sqlite3_errmsg(self.db);
                std::ffi::CStr::from_ptr(c_str)
                    .to_string_lossy()
                    .into_owned()
            };
            return Err(JsValue::from_str(&format!(
                "SQL preparation failed: {}",
                err_msg
            )));
        }

        let mut column_names = Vec::new();
        let col_count = unsafe { sqlite3_column_count(stmt) };

        for i in 0..col_count {
            let name = unsafe {
                let c_name = sqlite3_column_name(stmt, i);
                std::ffi::CStr::from_ptr(c_name)
                    .to_string_lossy()
                    .into_owned()
            };
            column_names.push(name);
        }

        let mut rows = Vec::new();

        loop {
            let step = unsafe { sqlite3_step(stmt) };
            if step == SQLITE_ROW {
                let mut row_data = Vec::new();
                for i in 0..col_count {
                    let val = unsafe {
                        let col_type = sqlite3_column_type(stmt, i);
                        match col_type {
                            SQLITE_INTEGER => {
                                serde_json::json!(sqlite3_column_int64(stmt, i))
                            }
                            SQLITE_FLOAT => {
                                serde_json::json!(sqlite3_column_double(stmt, i))
                            }
                            SQLITE_TEXT => {
                                let text = sqlite3_column_text(stmt, i);
                                if text.is_null() {
                                    serde_json::json!("")
                                } else {
                                    let s = std::ffi::CStr::from_ptr(text as *const i8)
                                        .to_string_lossy();
                                    serde_json::json!(s)
                                }
                            }
                            SQLITE_NULL => serde_json::json!(""),
                            _ => serde_json::json!(""), // Handle BLOBs if needed
                        }
                    };
                    row_data.push(val);
                }
                rows.push(row_data);
            } else if step == SQLITE_DONE {
                break;
            } else {
                unsafe { sqlite3_finalize(stmt) };
                return Err(JsValue::from_str("Error during query execution"));
            }
        }

        unsafe { sqlite3_finalize(stmt) };

        let result = QueryResult {
            columns: column_names,
            rows,
        };

        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))
    }

    #[wasm_bindgen]
    pub fn export_database(&self) -> Result<Vec<u8>, JsValue> {
        console_log!("Exporting database...");
        let mut size: i64 = 0;
        let c_main = CString::new("main").map_err(|_| JsValue::from_str("Invalid schema name"))?;

        // 0 flags = valid? usually 0 is generic.
        let ptr = unsafe { sqlite3_serialize(self.db, c_main.as_ptr(), &mut size, 0) };
        if ptr.is_null() {
            return Err(JsValue::from_str(
                "Failed to serialize database (ptr is null)",
            ));
        }

        let slice = unsafe { std::slice::from_raw_parts(ptr, size as usize) };
        let vec = slice.to_vec(); // Copy

        // Free the buffer allocated by sqlite3_serialize
        unsafe { sqlite3_free(ptr as *mut _) };

        console_log!("Database exported size: {} bytes", size);
        Ok(vec)
    }

    #[wasm_bindgen]
    pub fn import_database(&mut self, data: &[u8]) -> Result<(), JsValue> {
        console_log!("Importing database of size {} bytes...", data.len());

        // 1. Allocate buffer and copy data
        let size = data.len();
        let ptr = unsafe { sqlite3_malloc(size as i32) } as *mut u8;
        if ptr.is_null() {
            return Err(JsValue::from_str("Failed to allocate memory for import"));
        }

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), ptr, size);
        }

        // 2. Open NEW connection
        let mut new_db = ptr::null_mut();
        let c_filename =
            CString::new(":memory:").map_err(|_| JsValue::from_str("CString error"))?;
        let flags = SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_MEMORY;
        let ret = unsafe { sqlite3_open_v2(c_filename.as_ptr(), &mut new_db, flags, ptr::null()) };

        if ret != SQLITE_OK {
            unsafe { sqlite3_free(ptr as *mut _) };
            return Err(JsValue::from_str("Failed to open new database connection"));
        }

        // 3. Deserialize into NEW connection
        let c_main = CString::new("main").map_err(|_| JsValue::from_str("CString error"))?;
        let d_flags = SQLITE_DESERIALIZE_FREEONCLOSE | SQLITE_DESERIALIZE_RESIZEABLE;

        let ret = unsafe {
            sqlite3_deserialize(
                new_db,
                c_main.as_ptr(),
                ptr,
                size as i64,
                size as i64, // capacity
                d_flags as u32,
            )
        };

        if ret != SQLITE_OK {
            let err_msg = unsafe {
                let c_str = sqlite3_errmsg(new_db);
                std::ffi::CStr::from_ptr(c_str)
                    .to_string_lossy()
                    .into_owned()
            };
            unsafe {
                sqlite3_close(new_db);
                // Note: If deserialize fails, we must free the buffer ourselves.
                sqlite3_free(ptr as *mut _)
            };
            return Err(JsValue::from_str(&format!(
                "Failed to deserialize database: {}",
                err_msg
            )));
        }

        // 4. Close OLD connection and Swap
        unsafe { sqlite3_close(self.db) };
        self.db = new_db;

        console_log!("Database imported successfully and connection refreshed.");
        console_log!("Database imported successfully");
        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_documents(&self) -> Result<JsValue, JsValue> {
        let sql = "SELECT id, name, created_at FROM documents ORDER BY created_at DESC";
        self.execute_sql(sql)
    }

    fn insert_document(&self, name: &str, nodes: &[NodeData]) -> Result<u64, String> {
        // NOTE: A full transaction wrapper would be better, but doing simple EXEC for BEGIN/COMMIT here

        self.exec_internal("BEGIN TRANSACTION")?;

        let doc_id = match self.insert_doc_record(name) {
            Ok(id) => id,
            Err(e) => {
                let _ = self.exec_internal("ROLLBACK");
                return Err(e);
            }
        };

        // Map from parser local ID to database global ID
        let mut id_map: HashMap<i64, i64> = HashMap::new();

        for node in nodes {
            // Resolve parent ID using the map
            let db_parent_id = node.parent_id.and_then(|pid| id_map.get(&pid).copied());

            match self.insert_node_record(doc_id, node, db_parent_id) {
                Ok(new_id) => {
                    id_map.insert(node.id, new_id);
                }
                Err(e) => {
                    let _ = self.exec_internal("ROLLBACK");
                    return Err(e);
                }
            }
        }

        // Update root node
        if let Some(root) = nodes.first() {
            if let Some(&root_db_id) = id_map.get(&root.id) {
                let sql = format!(
                    "UPDATE documents SET root_node_id = {} WHERE id = {}",
                    root_db_id, doc_id
                );
                if let Err(e) = self.exec_internal(&sql) {
                    let _ = self.exec_internal("ROLLBACK");
                    return Err(e);
                }
            }
        }

        self.exec_internal("COMMIT")?;
        Ok(doc_id as u64)
    }

    fn exec_internal(&self, sql: &str) -> Result<(), String> {
        let c_sql = CString::new(sql).unwrap();
        let mut err_msg = ptr::null_mut();
        let ret =
            unsafe { sqlite3_exec(self.db, c_sql.as_ptr(), None, ptr::null_mut(), &mut err_msg) };

        if ret != SQLITE_OK {
            let msg = unsafe {
                if !err_msg.is_null() {
                    let s = std::ffi::CStr::from_ptr(err_msg)
                        .to_string_lossy()
                        .into_owned();
                    sqlite3_free(err_msg as *mut _);
                    s
                } else {
                    "Unknown error".to_string()
                }
            };
            return Err(msg);
        }
        Ok(())
    }

    fn insert_doc_record(&self, name: &str) -> Result<i64, String> {
        let sql = "INSERT INTO documents (name, root_node_id) VALUES (?, NULL)";
        let mut stmt = ptr::null_mut();
        let c_sql = CString::new(sql).unwrap();

        unsafe {
            if sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut())
                != SQLITE_OK
            {
                return Err("Failed to prepare insert doc".to_string());
            }
            // Bind name (index 1)
            let c_name = CString::new(name).unwrap();
            sqlite3_bind_text(stmt, 1, c_name.as_ptr(), -1, None);

            if sqlite3_step(stmt) != SQLITE_DONE {
                sqlite3_finalize(stmt);
                return Err("Failed to insert document".to_string());
            }

            sqlite3_finalize(stmt);
            Ok(sqlite3_last_insert_rowid(self.db))
        }
    }

    fn insert_node_record(
        &self,
        doc_id: i64,
        node: &NodeData,
        db_parent_id: Option<i64>,
    ) -> Result<i64, String> {
        // Allow ID to be autoincremented (pass NULL for id)
        let sql = "INSERT INTO nodes (id, document_id, parent_id, tag_name, text_content, depth, position) VALUES (NULL, ?, ?, ?, ?, ?, 0)";
        let mut stmt = ptr::null_mut();
        let c_sql = CString::new(sql).unwrap();

        let new_id = unsafe {
            if sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut())
                != SQLITE_OK
            {
                return Err("Failed to prepare node insert".to_string());
            }

            // Bind args
            // Index 1: document_id
            sqlite3_bind_int64(stmt, 1, doc_id);

            // Index 2: parent_id
            if let Some(pid) = db_parent_id {
                sqlite3_bind_int64(stmt, 2, pid);
            } else {
                sqlite_wasm_rs::sqlite3_bind_null(stmt, 2);
            }

            // Index 3: tag_name
            let c_tag = CString::new(node.tag_name.as_str()).unwrap();
            sqlite3_bind_text(stmt, 3, c_tag.as_ptr(), -1, None);

            // Index 4: text_content
            let c_text = if let Some(text) = &node.text_content {
                Some(CString::new(text.as_str()).unwrap())
            } else {
                None
            };

            if let Some(c) = &c_text {
                sqlite3_bind_text(stmt, 4, c.as_ptr(), -1, None);
            } else {
                sqlite_wasm_rs::sqlite3_bind_null(stmt, 4);
            }

            // Index 5: depth
            sqlite3_bind_int64(stmt, 5, node.depth as i64);

            if sqlite3_step(stmt) != SQLITE_DONE {
                sqlite3_finalize(stmt);
                return Err("Failed to insert node".to_string());
            }
            sqlite3_finalize(stmt);

            sqlite3_last_insert_rowid(self.db)
        };

        // attributes (use new_id)
        for (k, v) in &node.attributes {
            self.insert_attribute(new_id, k, v)?;
        }

        Ok(new_id)
    }

    fn insert_attribute(&self, node_id: i64, name: &str, value: &str) -> Result<(), String> {
        let sql = "INSERT INTO attributes (node_id, name, value) VALUES (?, ?, ?)";
        let mut stmt = ptr::null_mut();
        let c_sql = CString::new(sql).unwrap();

        unsafe {
            sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut());
            sqlite3_bind_int64(stmt, 1, node_id);
            let c_name = CString::new(name).unwrap();
            sqlite3_bind_text(stmt, 2, c_name.as_ptr(), -1, None);
            let c_val = CString::new(value).unwrap();
            sqlite3_bind_text(stmt, 3, c_val.as_ptr(), -1, None);

            let ret = sqlite3_step(stmt);
            sqlite3_finalize(stmt);
            if ret != SQLITE_DONE {
                return Err("Failed to insert attr".to_string());
            }
        }
        Ok(())
    }
}

// Re-implement init_schema to work with raw db pointer
fn init_schema_ffi(db: *mut sqlite3) -> Result<(), String> {
    let schema_sql = "
    CREATE TABLE IF NOT EXISTS documents (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        root_node_id INTEGER,
        created_at TEXT DEFAULT CURRENT_TIMESTAMP
    );

    CREATE TABLE IF NOT EXISTS nodes (
        id INTEGER PRIMARY KEY,
        document_id INTEGER NOT NULL,
        parent_id INTEGER,
        tag_name TEXT NOT NULL,
        text_content TEXT,
        depth INTEGER NOT NULL,
        position INTEGER NOT NULL,
        FOREIGN KEY (document_id) REFERENCES documents(id),
        FOREIGN KEY (parent_id) REFERENCES nodes(id)
    );

    CREATE TABLE IF NOT EXISTS attributes (
        id INTEGER PRIMARY KEY,
        node_id INTEGER NOT NULL,
        name TEXT NOT NULL,
        value TEXT,
        FOREIGN KEY (node_id) REFERENCES nodes(id)
    );
    ";

    let c_sql = CString::new(schema_sql).unwrap();
    let mut err_msg = ptr::null_mut();

    let ret = unsafe { sqlite3_exec(db, c_sql.as_ptr(), None, ptr::null_mut(), &mut err_msg) };

    if ret != SQLITE_OK {
        unsafe { sqlite3_free(err_msg as *mut _) };
        return Err("Failed to init schema".to_string());
    }
    Ok(())
}

// Additional FFI exports
use sqlite_wasm_rs::sqlite3_column_double;
use sqlite_wasm_rs::sqlite3_column_int64;

#[wasm_bindgen(start)]
pub fn main() {
    console_log!("WASM module loaded successfully");
}

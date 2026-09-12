use rusqlite::{Connection, Result};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct DbState {
    pub db: Mutex<Connection>,
}

// Struct for returning messages to the frontend
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DbMessage {
    pub id: i64,
    pub message_json: String,
}

pub fn init_db(app_handle: &AppHandle) -> Result<()> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    std::fs::create_dir_all(&app_dir).unwrap_or_default();

    let db_path = app_dir.join("kai_memory.db");
    let conn = Connection::open(&db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages_v2 (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_json TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tool_permissions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tool_name TEXT UNIQUE NOT NULL,
            status TEXT NOT NULL -- 'always_allow', 'ask', 'deny'
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fact TEXT UNIQUE NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS semantic_memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fact TEXT UNIQUE NOT NULL,
            embedding BLOB NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Manage state
    app_handle.manage(DbState {
        db: Mutex::new(conn),
    });

    Ok(())
}

#[tauri::command]
pub fn get_history(state: tauri::State<DbState>) -> Result<Vec<DbMessage>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT id, message_json FROM messages_v2 ORDER BY id ASC")
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(DbMessage {
                id: row.get(0)?,
                message_json: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut messages = Vec::new();
    for row in iter {
        if let Ok(msg) = row {
            messages.push(msg);
        }
    }

    Ok(messages)
}

#[tauri::command]
pub fn save_message(state: tauri::State<DbState>, message_json: String) -> Result<i64, String> {
    let db = state.db.lock().unwrap();
    db.execute(
        "INSERT INTO messages_v2 (message_json) VALUES (?1)",
        [&message_json],
    )
    .map_err(|e| e.to_string())?;

    Ok(db.last_insert_rowid())
}

#[tauri::command]
pub fn clear_history(state: tauri::State<DbState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db.execute("DELETE FROM messages_v2", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_core_memories(state: tauri::State<DbState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT fact FROM memories").map_err(|e| e.to_string())?;
    let iter = stmt.query_map([], |row| row.get(0)).map_err(|e| e.to_string())?;
    
    let mut memories = Vec::new();
    for row in iter {
        if let Ok(fact) = row {
            memories.push(fact);
        }
    }
    Ok(memories)
}

#[tauri::command]
pub fn save_core_memory(state: tauri::State<DbState>, fact: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db.execute(
        "INSERT OR IGNORE INTO memories (fact) VALUES (?1)",
        [&fact],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

async fn get_embedding(text: &str) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "nomic-embed-text",
        "prompt": text
    });
    
    let res = client.post("http://localhost:11434/api/embeddings")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    
    let embedding = json["embedding"]
        .as_array()
        .ok_or("No embedding found")?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect::<Vec<f32>>();
        
    Ok(embedding)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}

#[tauri::command]
pub async fn remember_semantic_fact(state: tauri::State<'_, DbState>, fact: String) -> Result<(), String> {
    let embedding = get_embedding(&fact).await?;
    
    // Convert Vec<f32> to bytes for SQLite BLOB
    let embedding_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            embedding.as_ptr() as *const u8,
            embedding.len() * std::mem::size_of::<f32>(),
        )
    };
    
    let db = state.db.lock().unwrap();
    db.execute(
        "INSERT OR REPLACE INTO semantic_memories (fact, embedding) VALUES (?1, ?2)",
        rusqlite::params![&fact, embedding_bytes],
    )
    .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn recall_semantic_memory(state: tauri::State<'_, DbState>, query: String) -> Result<Vec<String>, String> {
    let query_embedding = get_embedding(&query).await?;
    
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT fact, embedding FROM semantic_memories").map_err(|e| e.to_string())?;
    
    struct MemoryHit {
        fact: String,
        score: f32,
    }
    
    let iter = stmt.query_map([], |row| {
        let fact: String = row.get(0)?;
        let blob: Vec<u8> = row.get(1)?;
        Ok((fact, blob))
    }).map_err(|e| e.to_string())?;
    
    let mut hits = Vec::new();
    
    for row in iter {
        if let Ok((fact, blob)) = row {
            if blob.len() % std::mem::size_of::<f32>() == 0 {
                let floats: &[f32] = unsafe {
                    std::slice::from_raw_parts(
                        blob.as_ptr() as *const f32,
                        blob.len() / std::mem::size_of::<f32>(),
                    )
                };
                let score = cosine_similarity(&query_embedding, floats);
                hits.push(MemoryHit { fact, score });
            }
        }
    }
    
    // Sort by descending score
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    
    // Return top 3 results that have a reasonable similarity (e.g. > 0.4)
    let top_hits: Vec<String> = hits.into_iter()
        .filter(|h| h.score > 0.4)
        .take(3)
        .map(|h| h.fact)
        .collect();
        
    Ok(top_hits)
}

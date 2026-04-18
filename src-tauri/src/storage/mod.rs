use rusqlite::{params, Connection};
use std::sync::Mutex;
use once_cell::sync::OnceCell;
use crate::error::AppError;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use tauri::Manager;

static DB: OnceCell<Mutex<Connection>> = OnceCell::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct Meeting {
    pub id: String,
    pub title: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub locale: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub id: Option<i64>,
    pub meeting_id: String,
    pub t_start: f64,
    pub t_end: f64,
    pub text: String,
    pub is_final: bool,
}

pub fn init(app: &tauri::App) -> Result<(), AppError> {
    let app_dir = app.path().app_data_dir().map_err(|e| AppError::Generic(e.to_string()))?;
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir).map_err(|e| AppError::Generic(e.to_string()))?;
    }
    let db_path = app_dir.join("echomind.db");
    
    let conn = Connection::open(db_path).map_err(|e| AppError::Storage(e.to_string()))?;
    
    // Run migrations/schema
    let schema = include_str!("schema.sql");
    conn.execute_batch(schema).map_err(|e| AppError::Storage(e.to_string()))?;
    
    DB.set(Mutex::new(conn)).map_err(|_| AppError::Generic("Failed to set DB OnceCell".into()))?;
    
    Ok(())
}

fn get_conn() -> Result<std::sync::MutexGuard<'static, Connection>, AppError> {
    DB.get()
        .ok_or_else(|| AppError::Storage("Database not initialized".into()))?
        .lock()
        .map_err(|e| AppError::Generic(e.to_string()))
}

pub fn meeting_create(meeting: &Meeting) -> Result<(), AppError> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO meetings (id, title, started_at, locale) VALUES (?1, ?2, ?3, ?4)",
        params![meeting.id, meeting.title, meeting.started_at.to_rfc3339(), meeting.locale],
    ).map_err(|e| AppError::Storage(e.to_string()))?;
    Ok(())
}

pub fn meetings_list() -> Result<Vec<Meeting>, AppError> {
    let conn = get_conn()?;
    let mut stmt = conn.prepare("SELECT id, title, started_at, ended_at, locale FROM meetings ORDER BY started_at DESC")
        .map_err(|e| AppError::Storage(e.to_string()))?;
    
    let rows = stmt.query_map([], |row| {
        let started_at_str: String = row.get(2)?;
        let ended_at_str: Option<String> = row.get(3)?;
        
        Ok(Meeting {
            id: row.get(0)?,
            title: row.get(1)?,
            started_at: DateTime::parse_from_rfc3339(&started_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            ended_at: ended_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            locale: row.get(4)?,
        })
    }).map_err(|e| AppError::Storage(e.to_string()))?;
    
    let mut meetings = Vec::new();
    for row in rows {
        meetings.push(row.map_err(|e| AppError::Storage(e.to_string()))?);
    }
    Ok(meetings)
}

pub fn transcript_append(entry: &TranscriptEntry) -> Result<(), AppError> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO transcripts (meeting_id, t_start, t_end, text, is_final) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![entry.meeting_id, entry.t_start, entry.t_end, entry.text, if entry.is_final { 1 } else { 0 }],
    ).map_err(|e| AppError::Storage(e.to_string()))?;
    Ok(())
}

pub fn summary_save(meeting_id: &str, summary: &str) -> Result<(), AppError> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO summaries (meeting_id, created_at, summary) VALUES (?1, ?2, ?3)",
        params![meeting_id, Utc::now().to_rfc3339(), summary],
    ).map_err(|e| AppError::Storage(e.to_string()))?;
    Ok(())
}

pub fn meeting_get_transcript(meeting_id: &str) -> Result<Vec<TranscriptEntry>, AppError> {
    let conn = get_conn()?;
    let mut stmt = conn.prepare("SELECT id, meeting_id, t_start, t_end, text, is_final FROM transcripts WHERE meeting_id = ?1 ORDER BY t_start ASC")
        .map_err(|e| AppError::Storage(e.to_string()))?;
    
    let rows = stmt.query_map([meeting_id], |row| {
        Ok(TranscriptEntry {
            id: Some(row.get(0)?),
            meeting_id: row.get(1)?,
            t_start: row.get(2)?,
            t_end: row.get(3)?,
            text: row.get(4)?,
            is_final: row.get::<usize, i32>(5)? == 1,
        })
    }).map_err(|e| AppError::Storage(e.to_string()))?;
    
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(|e| AppError::Storage(e.to_string()))?);
    }
    Ok(entries)
}

pub fn meeting_get_latest_summary(meeting_id: &str) -> Result<Option<String>, AppError> {
    let conn = get_conn()?;
    let mut stmt = conn.prepare("SELECT summary FROM summaries WHERE meeting_id = ?1 ORDER BY created_at DESC LIMIT 1")
        .map_err(|e| AppError::Storage(e.to_string()))?;
    
    let mut rows = stmt.query([meeting_id]).map_err(|e| AppError::Storage(e.to_string()))?;
    if let Some(row) = rows.next().map_err(|e| AppError::Storage(e.to_string()))? {
        Ok(Some(row.get(0).map_err(|e| AppError::Storage(e.to_string()))?))
    } else {
        Ok(None)
    }
}

pub fn meeting_end(meeting_id: &str) -> Result<(), AppError> {
    let conn = get_conn()?;
    conn.execute(
        "UPDATE meetings SET ended_at = ?1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), meeting_id],
    ).map_err(|e| AppError::Storage(e.to_string()))?;
    Ok(())
}

pub fn setting_set(key: &str, value: &str) -> Result<(), AppError> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    ).map_err(|e| AppError::Storage(e.to_string()))?;
    Ok(())
}

pub fn setting_get(key: &str) -> Result<Option<String>, AppError> {
    let conn = get_conn()?;
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| AppError::Storage(e.to_string()))?;
    
    let mut rows = stmt.query([key]).map_err(|e| AppError::Storage(e.to_string()))?;
    if let Some(row) = rows.next().map_err(|e| AppError::Storage(e.to_string()))? {
        Ok(Some(row.get(0).map_err(|e| AppError::Storage(e.to_string()))?))
    } else {
        Ok(None)
    }
}

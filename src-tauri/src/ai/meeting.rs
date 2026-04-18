use std::sync::Mutex;
use once_cell::sync::Lazy;

pub struct MeetingState {
    pub current_meeting_id: Option<String>,
    pub transcript: Vec<String>,
    pub last_activity: std::time::Instant,
}

static STATE: Lazy<Mutex<MeetingState>> = Lazy::new(|| Mutex::new(MeetingState {
    current_meeting_id: None,
    transcript: Vec::new(),
    last_activity: std::time::Instant::now(),
}));

pub fn set_current_meeting_id(id: Option<String>) {
    let mut state = STATE.lock().unwrap();
    state.current_meeting_id = id;
}

pub fn get_current_meeting_id() -> Option<String> {
    let state = STATE.lock().unwrap();
    state.current_meeting_id.clone()
}

pub fn append_transcript(text: String) {
    let id = {
        let mut state = STATE.lock().unwrap();
        state.transcript.push(text.clone());
        state.last_activity = std::time::Instant::now();
        if state.transcript.len() > 100 {
            state.transcript.remove(0);
        }
        state.current_meeting_id.clone()
    };

    // Persist to DB if we have a meeting active
    if let Some(meeting_id) = id {
        let _ = crate::storage::transcript_append(&crate::storage::TranscriptEntry {
            id: None,
            meeting_id,
            t_start: 0.0, // Placeholder
            t_end: 0.0,   // Placeholder
            text,
            is_final: true,
        });
    }
}

pub fn get_full_transcript() -> String {
    let state = STATE.lock().unwrap();
    state.transcript.join(" ")
}

pub fn clear_transcript() {
    let mut state = STATE.lock().unwrap();
    state.transcript.clear();
    state.last_activity = std::time::Instant::now();
}

pub fn get_last_activity() -> std::time::Instant {
    let state = STATE.lock().unwrap();
    state.last_activity
}

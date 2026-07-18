#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod session;

use std::sync::{Arc, Mutex};

use session::ObserverSession;
use tauri::State;

struct ObserverState(Arc<Mutex<ObserverSession>>);

#[tauri::command]
fn observer_connect(request: Vec<u8>, state: State<'_, ObserverState>) -> Result<Vec<u8>, String> {
    with_session(&state, |session| session.connect(&request))
}

#[tauri::command]
fn observer_open_stream(state: State<'_, ObserverState>) -> Result<Vec<u8>, String> {
    with_session(&state, ObserverSession::open_runtime_stream)
}

#[tauri::command]
fn observer_advance(ticks: u64, state: State<'_, ObserverState>) -> Result<Vec<u8>, String> {
    with_session(&state, |session| session.advance(ticks))
}

#[tauri::command]
fn observer_query(request: Vec<u8>, state: State<'_, ObserverState>) -> Result<Vec<u8>, String> {
    with_session(&state, |session| session.query(&request))
}

#[tauri::command]
async fn observer_analyze(
    request: Vec<u8>,
    state: State<'_, ObserverState>,
) -> Result<Vec<u8>, String> {
    let session = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut session = session
            .lock()
            .map_err(|_| "observer session lock was poisoned".to_owned())?;
        session.analyze(&request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("observer analysis task failed: {error}"))?
}

#[tauri::command]
fn observer_reset(seed: u64, state: State<'_, ObserverState>) -> Result<Vec<u8>, String> {
    with_session(&state, |session| session.reset(seed))
}

fn with_session(
    state: &State<'_, ObserverState>,
    operation: impl FnOnce(&mut ObserverSession) -> Result<Vec<u8>, session::SessionError>,
) -> Result<Vec<u8>, String> {
    let mut session = state
        .0
        .lock()
        .map_err(|_| "observer session lock was poisoned".to_owned())?;
    operation(&mut session).map_err(|error| error.to_string())
}

fn main() {
    let session = ObserverSession::new(0).expect("default observer session must initialize");
    tauri::Builder::default()
        .manage(ObserverState(Arc::new(Mutex::new(session))))
        .invoke_handler(tauri::generate_handler![
            observer_connect,
            observer_open_stream,
            observer_advance,
            observer_query,
            observer_analyze,
            observer_reset,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Causafera Observer");
}

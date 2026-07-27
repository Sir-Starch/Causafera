#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod session;

use std::sync::{Arc, Mutex};

use session::ObserverSession;
use tauri::{ipc::Response, State};

struct ObserverState(Arc<Mutex<ObserverSession>>);

/// Every command answers with the protocol's own bytes, sent as an IPC
/// [`Response`] rather than a `Vec<u8>` return value. Tauri serialises an
/// ordinary `Vec<u8>` return as a JSON array of numbers — for a 3.3 KB
/// terrain raster that is four times the bytes on the wire, encoded and
/// decoded as JSON on every call. `Response` carries the same bytes raw.
type CommandResult = Result<Response, String>;

#[tauri::command]
fn observer_connect(request: Vec<u8>, state: State<'_, ObserverState>) -> CommandResult {
    with_session(&state, |session| session.connect(&request))
}

#[tauri::command]
fn observer_open_stream(state: State<'_, ObserverState>) -> CommandResult {
    with_session(&state, ObserverSession::open_runtime_stream)
}

#[tauri::command]
fn observer_advance(ticks: u64, state: State<'_, ObserverState>) -> CommandResult {
    with_session(&state, |session| session.advance(ticks))
}

#[tauri::command]
fn observer_query(request: Vec<u8>, state: State<'_, ObserverState>) -> CommandResult {
    with_session(&state, |session| session.query(&request))
}

#[tauri::command]
async fn observer_analyze(request: Vec<u8>, state: State<'_, ObserverState>) -> CommandResult {
    let session = Arc::clone(&state.0);
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        let mut session = session
            .lock()
            .map_err(|_| "observer session lock was poisoned".to_owned())?;
        session.analyze(&request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("observer analysis task failed: {error}"))??;
    Ok(Response::new(bytes))
}

/// One chunk of one measured lattice. The request travels as protocol bytes
/// like every other query, so the boundary stays one shape.
#[tauri::command]
fn observer_field_raster(request: Vec<u8>, state: State<'_, ObserverState>) -> CommandResult {
    with_session(&state, |session| session.query(&request))
}

#[tauri::command]
fn observer_reset(seed: u64, state: State<'_, ObserverState>) -> CommandResult {
    with_session(&state, |session| session.reset(seed))
}

fn with_session(
    state: &State<'_, ObserverState>,
    operation: impl FnOnce(&mut ObserverSession) -> Result<Vec<u8>, session::SessionError>,
) -> CommandResult {
    let mut session = state
        .0
        .lock()
        .map_err(|_| "observer session lock was poisoned".to_owned())?;
    let bytes = operation(&mut session).map_err(|error| error.to_string())?;
    Ok(Response::new(bytes))
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
            observer_field_raster,
            observer_analyze,
            observer_reset,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Causafera Observer");
}

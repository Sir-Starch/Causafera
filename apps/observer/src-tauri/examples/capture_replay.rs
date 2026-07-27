//! Development-only capture of real observer protocol bytes.
//!
//! This example drives the same `ObserverSession` the desktop shell uses and writes the
//! byte-identical protocol frames it produced to a JSON file. The output is authentic
//! deterministic runtime output, not fixture or demo data, and it is never linked into the
//! desktop binary. It exists so frontend work can be verified against real observer payloads
//! without a graphical session.
//!
//! ```text
//! cargo run -p causafera-observer --example capture_replay -- <output.json> [seed] [frames] [ticks-per-frame]
//! ```

// The capture drives the desktop shell's own session module; `reset` is unused here.
#[allow(dead_code)]
#[path = "../src/session.rs"]
mod session;

use std::fmt::Write as _;

use causafera_observer_api::{
    FieldRasterKind, FieldRasterRequest, OBSERVER_PROTOCOL_V1, ObserverQuery, QueryKind,
};
use causafera_observer_wire::{
    ConnectRequest, decode_response, decode_world_snapshot, encode_connect_request,
    encode_field_raster_request, encode_query,
};

use session::ObserverSession;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().unwrap_or_else(|| {
        eprintln!("usage: capture_replay <output.json> [seed] [frames] [ticks-per-frame]");
        std::process::exit(2);
    });
    let seed = parse(arguments.next(), 7);
    let frames = parse(arguments.next(), 24);
    let ticks_per_frame = parse(arguments.next(), 2);

    let mut session = ObserverSession::new(seed).expect("observer session must initialize");
    let connect = session
        .connect(&encode_connect_request(&ConnectRequest {
            supported_versions: vec![OBSERVER_PROTOCOL_V1],
            locale: "ru-RU".into(),
        }))
        .expect("connect must negotiate");

    let mut captured = Vec::with_capacity(frames as usize + 1);
    let opened = session
        .open_runtime_stream()
        .expect("runtime stream must open");
    let world = world_query(&mut session, 1);
    let chunks = chunk_scopes(&world);
    captured.push(CapturedFrame {
        ticks: 0,
        runtime: opened,
        rasters: rasters(&mut session, &chunks, 10_000, true),
        world,
    });

    for index in 0..frames {
        let runtime = session
            .advance(ticks_per_frame)
            .expect("advance must succeed");
        let world = world_query(&mut session, index + 2);
        captured.push(CapturedFrame {
            ticks: (index + 1) * ticks_per_frame,
            runtime,
            rasters: rasters(&mut session, &chunks, 10_000 + (index + 1) * 100, false),
            world,
        });
    }

    let explanation = session
        .analyze(&encode_query(&ObserverQuery {
            request_id: 9_000,
            protocol_version: OBSERVER_PROTOCOL_V1,
            kind: QueryKind::ExplanationIr,
            scope: None,
            payload: Vec::new(),
        }))
        .expect("explanation query must succeed");

    let mut json = String::with_capacity(1 << 16);
    json.push_str("{\n");
    writeln!(json, "  \"seed\": {seed},").unwrap();
    writeln!(json, "  \"ticksPerFrame\": {ticks_per_frame},").unwrap();
    write!(json, "  \"connect\": ").unwrap();
    write_bytes(&mut json, &connect);
    json.push_str(",\n  \"frames\": [\n");
    for (index, frame) in captured.iter().enumerate() {
        json.push_str("    { \"ticks\": ");
        write!(json, "{}", frame.ticks).unwrap();
        json.push_str(", \"runtime\": ");
        write_bytes(&mut json, &frame.runtime);
        json.push_str(", \"world\": ");
        write_bytes(&mut json, &frame.world);
        json.push_str(", \"rasters\": {");
        for (index, (key, bytes)) in frame.rasters.iter().enumerate() {
            if index > 0 {
                json.push(',');
            }
            write!(json, "\"{key}\": ").unwrap();
            write_bytes(&mut json, bytes);
        }
        json.push_str("} }");
        if index + 1 < captured.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n  \"explanation\": ");
    write_bytes(&mut json, &explanation);
    json.push_str("\n}\n");

    std::fs::write(&path, json).expect("capture output must be writable");
    eprintln!(
        "captured {} frames (seed {seed}, {ticks_per_frame} ticks per frame) to {path}",
        captured.len()
    );
}

struct CapturedFrame {
    ticks: u64,
    runtime: Vec<u8>,
    world: Vec<u8>,
    /// Keyed exactly as the frontend keys its raster cache, so a replay can
    /// answer a request without the channel having to model the runtime.
    rasters: Vec<(String, Vec<u8>)>,
}

fn world_query(session: &mut ObserverSession, request_id: u64) -> Vec<u8> {
    session
        .query(&encode_query(&ObserverQuery::world_chunks(request_id)))
        .expect("world query must succeed")
}

fn chunk_scopes(world: &[u8]) -> Vec<(u64, i32, i32, i32)> {
    decode_world_snapshot(
        &decode_response(world)
            .expect("world response must decode")
            .payload,
    )
    .expect("world snapshot must decode")
    .chunks
    .iter()
    .map(|chunk| (chunk.chart_id, chunk.chunk_x, chunk.chunk_y, chunk.chunk_z))
    .collect()
}

/// Every field the map draws, for every chunk the observer received.
///
/// Terrain is generated once and does not change per tick, so it is captured
/// with the first frame only and the replay falls back to it — twenty-five
/// copies of an identical lattice is a larger capture, not a better one.
fn rasters(
    session: &mut ObserverSession,
    chunks: &[(u64, i32, i32, i32)],
    mut request_id: u64,
    include_static: bool,
) -> Vec<(String, Vec<u8>)> {
    let mut captured = Vec::new();
    let fields: &[FieldRasterKind] = if include_static {
        &[
            FieldRasterKind::TerrainElevation,
            FieldRasterKind::ManaIntensity,
        ]
    } else {
        &[FieldRasterKind::ManaIntensity]
    };
    for (chart_id, chunk_x, chunk_y, chunk_z) in chunks.iter().copied() {
        for field in fields.iter().copied() {
            request_id += 1;
            let response = session
                .query(&encode_query(&ObserverQuery::field_raster(
                    request_id,
                    encode_field_raster_request(&FieldRasterRequest {
                        chart_id,
                        chunk_x,
                        chunk_y,
                        chunk_z,
                        field,
                        detail_level: 0,
                    }),
                )))
                .expect("raster query must succeed");
            captured.push((
                format!("{}|{chart_id}:{chunk_x}:{chunk_y}:{chunk_z}", field as u32),
                response,
            ));
        }
    }
    captured
}

fn write_bytes(json: &mut String, bytes: &[u8]) {
    json.push('[');
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write!(json, "{byte}").unwrap();
    }
    json.push(']');
}

fn parse(value: Option<String>, fallback: u64) -> u64 {
    value
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

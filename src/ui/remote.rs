//! The viewer's control socket. The MCP server (`ray mcp`, src/mcp.rs) sends
//! commands to it, one JSON object per line ({"cmd": ..., "args": {...}}),
//! and gets one JSON line back ({"ok": true, "result": ...} or {"ok": false,
//! "error": ...}). The connection threads hand each command to the UI
//! thread, which carries it out on its next frame (control.rs).

use eframe::egui;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

/// A command from a connection, and where its answer goes.
pub struct Request {
    pub cmd: String,
    pub args: Value,
    pub reply: Sender<Result<Value, String>>,
}

pub struct Remote {
    pub rx: Receiver<Request>,
    path: PathBuf,
}

impl Remote {
    /// Listens on the control socket, unless another viewer already does.
    pub fn start(ctx: egui::Context) -> Option<Remote> {
        let path = crate::mcp::socket_path();
        if UnixStream::connect(&path).is_ok() {
            eprintln!("ray: another viewer has the control socket {}; this one takes no commands", path.display());
            return None;
        }
        // a socket file nobody listens on is left from a viewer that ended
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).map_err(|e| eprintln!("ray: control socket {}: {e}", path.display())).ok()?;
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("control".into())
            .spawn(move || {
                for conn in listener.incoming().flatten() {
                    let (tx, ctx) = (tx.clone(), ctx.clone());
                    let _ = std::thread::Builder::new().name("control conn".into()).spawn(move || serve(conn, tx, ctx));
                }
            })
            .ok()?;
        Some(Remote { rx, path })
    }
}

impl Drop for Remote {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Answers one connection's commands in turn.
fn serve(conn: UnixStream, tx: Sender<Request>, ctx: egui::Context) {
    let Ok(read) = conn.try_clone() else { return };
    let mut out = conn;
    for line in BufReader::new(read).lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(v) => {
                let cmd = v.get("cmd").and_then(Value::as_str).unwrap_or("").to_string();
                let args = v.get("args").cloned().unwrap_or_else(|| json!({}));
                let (rtx, rrx) = channel();
                if tx.send(Request { cmd, args, reply: rtx }).is_err() {
                    break;
                }
                ctx.request_repaint();
                // answered on a later frame (a screenshot waits for the image)
                match rrx.recv_timeout(Duration::from_secs(120)) {
                    Ok(Ok(r)) => json!({"ok": true, "result": r}),
                    Ok(Err(e)) => json!({"ok": false, "error": e}),
                    Err(_) => json!({"ok": false, "error": "the viewer did not answer (is its window minimised?)"}),
                }
            }
            Err(e) => json!({"ok": false, "error": format!("bad request: {e}")}),
        };
        if writeln!(out, "{reply}").is_err() {
            break;
        }
    }
}

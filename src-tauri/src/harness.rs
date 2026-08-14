//! Spawn and supervise the DeepSeek Harness (dsh) local web server.
//!
//! The desktop shell stays intentionally "dumb": the only coupling points
//! to the harness are the CLI entry path, the four spawn flags, and a TCP
//! probe. We never parse its stdout — an upstream update can only break the
//! spawn flags, which is a one-line change.

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use tauri::Manager;
use url::Url;

const POLL_INTERVAL: Duration = Duration::from_millis(150);
const READY_TIMEOUT: Duration = Duration::from_secs(30);

/// PID of the spawned node process, stored so the exit path can kill the tree.
static CHILD_PID: AtomicU32 = AtomicU32::new(0);
/// Set when *we* asked for the child to die (app exiting). The supervisor
/// uses it to skip showing the error page for a deliberate shutdown.
static INTENTIONAL_KILL: AtomicBool = AtomicBool::new(false);

/// Project root = parent of `src-tauri` (the repo checkout).
pub fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has a parent directory")
}

/// Resolve the dsh CLI entry. `DSH_CMD` env var wins; otherwise the
/// project-local npm dependency `node_modules/@deepseek-ai/dsh/lib/bin.js`.
pub fn dsh_cmd_path() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("DSH_CMD") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!("DSH_CMD is set but does not exist: {}", p.display()));
    }
    let candidate = project_root().join("node_modules/@deepseek-ai/dsh/lib/bin.js");
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(format!(
            "dsh CLI not found at {} — run `npm install` in the project root",
            candidate.display()
        ))
    }
}

/// Bind an ephemeral port and hand the number back (we then spawn the
/// harness on it — never the hardcoded 3080, so a manually-run `dsh web`
/// cannot collide).
pub fn find_free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

/// Resolve the Node executable. Prefer the bundled portable runtime under
/// `node-runtime/` (kept at a version matching the harness's dependency floor
/// of node >= 22.19, per `@earendil-works/pi-ai`), and fall back to whatever
/// `node` resolves to on PATH.
pub fn node_executable() -> PathBuf {
    let local = project_root().join("node-runtime").join("node.exe");
    if local.is_file() {
        return local;
    }
    PathBuf::from("node")
}

/// Spawn `node <dsh bin.js> web --host 127.0.0.1 --port <port>`.
///
/// stdout/stderr go to `dsh-gui-node.log` — a file, not a pipe. The OS pipe
/// buffer is only ~64 KB; node would block once it fills if nothing drains it.
pub fn spawn(port: u16) -> Result<Child, String> {
    let bin = dsh_cmd_path()?;
    let log = std::fs::File::create(project_root().join("dsh-gui-node.log"))
        .map_err(|e| e.to_string())?;
    let err_log = log.try_clone().map_err(|e| e.to_string())?;
    let mut cmd = Command::new(node_executable());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // node.exe is a console-subsystem binary; spawned from a GUI app it
        // would otherwise pop a fresh terminal window. Hide it.
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.arg(&bin)
        .arg("web")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .current_dir(project_root())
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err_log))
        .spawn()
        .map_err(|e| format!("failed to spawn node (is Node.js on PATH?): {e}"))
}

/// Poll TCP on 127.0.0.1:port until it accepts, the child exits early,
/// or the timeout elapses.
pub fn wait_ready(port: u16, child: &mut Child) -> Result<(), String> {
    let deadline = std::time::Instant::now() + READY_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Err(format!("dsh exited early ({status})"));
        }
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(format!("timed out waiting for dsh on 127.0.0.1:{port}"));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Build a `data:` URL from the embedded `error.html` so we never have to
/// guess the app-origin scheme (`tauri://` vs `http://tauri.localhost`) to
/// navigate back to a bundled asset.
pub fn error_url() -> Url {
    let html = include_str!("../../src-dist/error.html");
    let encoded = percent_encoding::utf8_percent_encode(html, percent_encoding::NON_ALPHANUMERIC);
    Url::parse(&format!("data:text/html;charset=utf-8,{encoded}")).expect("valid error URL")
}

/// Background supervisor: wait for the harness, navigate the main window to
/// it, then watch the child for an unexpected exit and surface the error page.
pub fn supervise(port: u16, mut child: Child, app: tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };
    match wait_ready(port, &mut child) {
        Ok(()) => {
            let url = Url::parse(&format!("http://127.0.0.1:{port}/")).expect("valid URL");
            println!("dsh ready at {url}");
            let _ = win.navigate(url);
            let _ = child.wait();
            if !INTENTIONAL_KILL.load(Ordering::SeqCst) {
                let _ = win.navigate(error_url());
            }
        }
        Err(e) => {
            eprintln!("harness failed to start: {e}");
            if !INTENTIONAL_KILL.load(Ordering::SeqCst) {
                let _ = win.navigate(error_url());
            }
            let _ = child.kill();
        }
    }
}

pub fn record_pid(pid: u32) {
    CHILD_PID.store(pid, Ordering::SeqCst);
}

/// Kill the whole node process tree. `Child::kill()` only takes down the
/// direct child, so use `taskkill /T /F` on Windows. Idempotent.
pub fn kill_tree() {
    let pid = CHILD_PID.load(Ordering::SeqCst);
    if pid == 0 {
        return;
    }
    INTENTIONAL_KILL.store(true, Ordering::SeqCst);
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

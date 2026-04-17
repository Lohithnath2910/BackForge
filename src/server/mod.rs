use std::process::{Child, Command, Stdio};
use std::thread;
use std::io::{BufRead, BufReader};
use std::fs;
use std::path::PathBuf;
use crate::core::models::Project;
use crate::export::export_project;

/// Finds Python: checks active venv first, then conda, then PATH.
pub fn find_python() -> Option<String> {
    // 1. Active venv
    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        let base = PathBuf::from(&venv);
        for p in &[base.join("Scripts").join("python.exe"), base.join("bin").join("python")] {
            if p.exists() { return Some(p.to_string_lossy().into_owned()); }
        }
    }
    // 2. Conda
    if let Ok(conda) = std::env::var("CONDA_PREFIX") {
        let base = PathBuf::from(&conda);
        for p in &[base.join("python.exe"), base.join("bin").join("python")] {
            if p.exists() { return Some(p.to_string_lossy().into_owned()); }
        }
    }
    // 3. PATH
    for c in &["python3", "python", "python3.12", "python3.11"] {
        if which::which(c).is_ok() { return Some(c.to_string()); }
    }
    None
}

/// The launcher script written into the temp dir.
const LAUNCHER: &str = r#"
import sys
import os

# Ensure the project directory is on the path so uvicorn can import main:app
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# Windows fix: use SelectorEventLoop (fixes Windows Store Python 3.12 crash)
if sys.platform == "win32":
    import asyncio
    asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())

try:
    import uvicorn
    import fastapi
    import sqlalchemy
except ImportError as e:
    print(f"[backforge] Missing package: {e}")
    print("[backforge] Run: pip install fastapi uvicorn[standard] sqlalchemy")
    sys.exit(1)

# Try importing the app first so any syntax errors show clearly
try:
    import main as _test_main
    del _test_main
except Exception as e:
    print(f"[backforge] ERROR importing main.py: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)

uvicorn.run(
    "main:app",
    host="127.0.0.1",
    port=PORT_PLACEHOLDER,
    log_level="info",
    loop="asyncio",
)
"#;

pub fn start_server(
    project: &Project,
    port: u16,
    log_tx: std::sync::mpsc::Sender<String>,
) -> Result<Child, String> {
    let python = find_python().ok_or_else(|| {
        "Python not found. Activate your venv first, then press [s] again.".to_string()
    })?;

    let temp_dir = std::env::temp_dir().join("backforge_server");
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // Write the FastAPI project
    export_project(project, temp_dir.to_str().unwrap())
        .map_err(|e| format!("Export failed: {}", e))?;

    // Write the Windows-safe launcher script
    let launcher_code = LAUNCHER.replace("PORT_PLACEHOLDER", &port.to_string());
    fs::write(temp_dir.join("_run.py"), &launcher_code)
        .map_err(|e| format!("Could not write launcher: {}", e))?;

    log_tx.send(format!("[backforge] Project written to: {}", temp_dir.display())).ok();
    log_tx.send(format!("[backforge] Python: {}", python)).ok();

    // Install deps silently using the exact same python binary
    log_tx.send("[backforge] Installing deps...".to_string()).ok();
    let pip = Command::new(&python)
        .args(["-m", "pip", "install", "-q",
               "fastapi", "uvicorn[standard]", "sqlalchemy",
               "passlib[bcrypt]", "python-jose[cryptography]",
               "python-multipart", "pydantic-settings"])
        .current_dir(&temp_dir)
        .output();
    match pip {
        Ok(o) if !o.status.success() => {
            log_tx.send(format!("[pip] {}", String::from_utf8_lossy(&o.stderr).trim())).ok();
        }
        Err(e) => { log_tx.send(format!("[pip error] {}", e)).ok(); }
        _ => { log_tx.send("[backforge] Deps OK. Starting server...".to_string()).ok(); }
    }

    log_tx.send(format!("[backforge] Starting on http://127.0.0.1:{}/docs", port)).ok();

    // Run via _run.py instead of `python -m uvicorn` — avoids CLI/multiprocessing issues
    let mut child = Command::new(&python)
        .arg("_run.py")
        .current_dir(&temp_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start server: {}", e))?;

    if let Some(stderr) = child.stderr.take() {
        let tx = log_tx.clone();
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                let _ = tx.send(line);
            }
        });
    }
    if let Some(stdout) = child.stdout.take() {
        let tx = log_tx;
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().flatten() {
                let _ = tx.send(line);
            }
        });
    }

    Ok(child)
}

pub fn check_python_deps() -> Vec<String> {
    let mut missing = Vec::new();
    if find_python().is_none() {
        missing.push("Python not found. Activate your venv then restart backforge.".to_string());
    }
    missing
}

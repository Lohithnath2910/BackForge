mod core;
mod ui;
mod export;
mod server;

use std::io;
use std::process::Child;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::path::PathBuf;
use std::fs;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use ui::events::{handle_key, AppAction, FireRequest};
use ui::renderer::render;
use ui::state::{AppState, ServerStatus, TesterResponse};
use server::start_server;
use export::export_project;
use core::models::Project;

// ─── Persistence ──────────────────────────────────────────────────────────────

fn projects_dir() -> PathBuf {
    let base = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    base.join("backforge_projects")
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .ok()
}

fn safe_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

fn save_project(project: &Project) -> anyhow::Result<PathBuf> {
    let dir = projects_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", safe_filename(&project.name)));
    let json = serde_json::to_string_pretty(project)?;
    fs::write(&path, &json)?;
    Ok(path)
}

fn load_project_by_name(name: &str) -> Option<Project> {
    let dir = projects_dir();
    let path = dir.join(format!("{}.json", safe_filename(name)));
    let json = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}

fn delete_project_file(name: &str) -> bool {
    let dir = projects_dir();
    let path = dir.join(format!("{}.json", safe_filename(name)));
    fs::remove_file(&path).is_ok()
}

/// Returns list of (display_name, file_path) by reading the actual JSON name field
fn list_saved_projects() -> Vec<(String, PathBuf)> {
    let dir = projects_dir();
    let Ok(entries) = fs::read_dir(&dir) else { return vec![] };
    let mut projects: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension()?.to_str()? != "json" { return None; }
            // Read the actual name from JSON so we don't mangle it
            let json = fs::read_to_string(&path).ok()?;
            let val: serde_json::Value = serde_json::from_str(&json).ok()?;
            let name = val.get("name")?.as_str()?.to_string();
            Some((name, path))
        })
        .collect();
    projects.sort_by(|a, b| a.0.cmp(&b.0));
    projects
}

// ─── Project Selection Menu (pre-raw-mode) ────────────────────────────────────

fn select_project() -> (String, Option<Project>) {
    loop {
        let saved = list_saved_projects();
        let save_dir = projects_dir();

        println!("╔══════════════════════════════════════════════════════╗");
        println!("║  ██████╗  █████╗  ██████╗██╗  ██╗                  ║");
        println!("║  ██╔══██╗██╔══██╗██╔════╝██║ ██╔╝                  ║");
        println!("║  ██████╔╝███████║██║     █████╔╝                   ║");
        println!("║  ██╔══██╗██╔══██║██║     ██╔═██╗                   ║");
        println!("║  ██████╔╝██║  ██║╚██████╗██║  ██╗                  ║");
        println!("║  ╚═════╝ ╚════╝  ╚═════╝╚═╝  ╚═╝                  ║");
        println!("║  ███████╗ ██████╗ ██████╗  ██████╗ ███████╗        ║");
        println!("║  ██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝        ║");
        println!("║  █████╗  ██║   ██║██████╔╝██║  ███╗█████╗          ║");
        println!("║  ██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝          ║");
        println!("║  ██║     ╚██████╔╝██║  ██║╚██████╔╝███████╗        ║");
        println!("║  ╚═╝      ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝       ║");
        println!("╚══════════════════════════════════════════════════════╝");
        println!("  TUI Backend Generator & Playground");
        println!("  Projects folder: {}\n", save_dir.display());

        if saved.is_empty() {
            println!("  No saved projects yet.\n");
        } else {
            println!("  Saved projects:");
            for (i, (name, _)) in saved.iter().enumerate() {
                println!("    [{:>2}] {}", i + 1, name);
            }
            println!();
        }

        println!("  [n]  New project");
        if !saved.is_empty() {
            println!("  [d#] Delete project  (e.g. d1, d2 ...)");
        }
        println!("  [q]  Quit\n");

        print!("  Select: ");
        use std::io::Write;
        let _ = io::stdout().flush();

        let mut input = String::new();
        let _ = io::stdin().read_line(&mut input);
        let input = input.trim().to_string();

        match input.as_str() {
            "q" => std::process::exit(0),
            "n" | "" => break,
            s if s.starts_with('d') || s.starts_with('D') => {
                let num_part = &s[1..];
                if let Ok(idx) = num_part.parse::<usize>() {
                    if idx >= 1 && idx <= saved.len() {
                        let (name, path) = &saved[idx - 1];
                        println!("\n  Delete '{}' at {}?", name, path.display());
                        print!("  [y] Yes  [n] No: ");
                        let _ = io::stdout().flush();
                        let mut confirm = String::new();
                        let _ = io::stdin().read_line(&mut confirm);
                        if confirm.trim().to_lowercase() == "y" {
                            if delete_project_file(name) {
                                println!("  ✓ Deleted '{}'\n", name);
                            } else {
                                println!("  ✗ Could not delete. Try manually at:\n    {}\n", path.display());
                            }
                        } else {
                            println!("  Cancelled.\n");
                        }
                    } else {
                        println!("  Invalid number.\n");
                    }
                } else {
                    println!("  Usage: d1, d2, d3 ... to delete by number.\n");
                }
                continue;
            }
            s => {
                if let Ok(idx) = s.parse::<usize>() {
                    if idx >= 1 && idx <= saved.len() {
                        let (name, _) = &saved[idx - 1];
                        let name = name.clone();
                        let project = load_project_by_name(&name);
                        if project.is_some() {
                            println!("\n  ✓ Loaded '{}'\n", name);
                            return (name, project);
                        } else {
                            println!("  ✗ Could not load '{}' (corrupted?)\n", name);
                            continue;
                        }
                    }
                }
                let matched = saved.iter().find(|(n, _)| n.to_lowercase().contains(&s.to_lowercase()));
                if let Some((name, _)) = matched {
                    let name = name.clone();
                    let project = load_project_by_name(&name);
                    if project.is_some() {
                        println!("\n  ✓ Loaded '{}'\n", name);
                        return (name, project);
                    }
                }
                println!("  Not found.\n");
                continue;
            }
        }
    }

    // New project name
    print!("  New project name (default: MyProject): ");
    use std::io::Write;
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    let name = input.trim().to_string();
    let name = if name.is_empty() { "MyProject".to_string() } else { name };
    println!("  ✓ Starting new project '{}'\n", name);
    (name, None)
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (project_name, saved_project) = if args.len() > 1 && !args[1].starts_with('-') {
        let name = args[1].clone();
        let project = load_project_by_name(&name);
        if project.is_some() {
            println!("✓ Loaded project '{}'", name);
        } else {
            println!("Starting new project '{}'", name);
        }
        (name, project)
    } else {
        select_project()
    };

    // ── Terminal setup ───────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── App state ────────────────────────────────────────────────────────────
    let mut state = AppState::new(project_name.clone());
    if let Some(proj) = saved_project {
        state.project = proj;
    }

    let mut server_child: Option<Child> = None;
    let (log_tx, log_rx) = mpsc::channel::<String>();
    let (server_start_tx, server_start_rx) = mpsc::channel::<Result<Child, String>>();

    // ── Main loop ────────────────────────────────────────────────────────────
    loop {
        state.tick_notification();

        // Drain server logs
        while let Ok(line) = log_rx.try_recv() {
            if line.contains("Application startup complete") || line.contains("Uvicorn running") {
                if let ServerStatus::Starting = state.server_status {
                    state.server_status = ServerStatus::Running { port: state.server_port };
                }
            }
            state.server_logs.push(line);
            if state.server_logs.len() > 200 {
                state.server_logs.remove(0);
            }
        }

        // Check if server process died
        if let Some(ref mut child) = server_child {
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    state.server_logs.push(format!("Server exited: {}", exit_status));
                    state.server_status = ServerStatus::Stopped;
                    server_child = None;
                }
                Ok(None) => {}
                Err(_) => {}
            }
        }

        // Receive async server startup result
        while let Ok(start_result) = server_start_rx.try_recv() {
            match start_result {
                Ok(child) => {
                    server_child = Some(child);
                    state.notify(format!("Server starting on :{} ...", state.server_port));
                }
                Err(e) => {
                    state.server_status = ServerStatus::Error(e.clone());
                    state.server_logs.push(format!("ERROR: {}", e));
                }
            }
        }

        terminal.draw(|f| render(f, &state))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let action = handle_key(&mut state, key);
                match action {
                    AppAction::Quit => break,
                    AppAction::StartServer => {
                        if !matches!(state.server_status, ServerStatus::Stopped) {
                            continue;
                        }
                        state.server_status = ServerStatus::Starting;
                        state.server_logs.clear();
                        let missing = server::check_python_deps();
                        if !missing.is_empty() {
                            let msg = missing.join(", ");
                            state.server_status = ServerStatus::Error(msg.clone());
                            state.server_logs.push(format!("ERROR: {}", msg));
                        } else {
                            let project = state.project.clone();
                            let port = state.server_port;
                            let log_sender = log_tx.clone();
                            let result_sender = server_start_tx.clone();
                            thread::spawn(move || {
                                let result = start_server(&project, port, log_sender);
                                let _ = result_sender.send(result);
                            });
                        }
                    }
                    AppAction::StopServer => {
                        if let Some(ref mut child) = server_child {
                            let _ = child.kill();
                        }
                        server_child = None;
                        state.server_status = ServerStatus::Stopped;
                        state.notify("Server stopped.".to_string());
                    }
                    AppAction::TestEndpoint { endpoint_id } => {
                        if let Some(idx) = state.project.endpoints.iter().position(|e| e.id == endpoint_id) {
                            if state.server_status == ServerStatus::Stopped {
                                state.notify("Start server first → [s] → [s]".to_string());
                            } else {
                                state.open_tester(idx);
                            }
                        }
                    }
                    AppAction::SendRequest(req) => {
                        state.tester.loading = true;
                        state.tester.response = None;
                        let method    = req.method.clone().to_uppercase();
                        let body_kvs  = req.body_kvs.clone();
                        let path_kvs  = req.path_kvs.clone();
                        let result    = fire_http_request(req);
                        state.tester.loading = false;

                        // ── Sync fake DB based on HTTP result ─────────────────
                        let ep_idx = state.tester.endpoint_idx;
                        if let Some(ep) = state.project.endpoints.get(ep_idx).cloned() {
                            if let Some(model_id) = ep.linked_model.clone() {
                                let s = result.status;

                                match method.as_str() {
                                    // CREATE → add row; use id from response JSON if present
                                    "POST" if s == 200 || s == 201 => {
                                        // Try to parse the real id from the JSON response
                                        let real_id: Option<u64> = serde_json::from_str::<serde_json::Value>(&result.body)
                                            .ok()
                                            .and_then(|v| v.get("id")
                                                .and_then(|id| id.as_u64()));

                                        if let Some(table) = state.project.get_fake_table_mut(&model_id) {
                                            let row_id = real_id.unwrap_or_else(|| {
                                                let id = table.next_id;
                                                table.next_id += 1;
                                                id
                                            });
                                            // Keep next_id ahead of any real id
                                            if row_id >= table.next_id {
                                                table.next_id = row_id + 1;
                                            }
                                            // Remove stale row with same id if exists (re-create)
                                            table.rows.retain(|r| r.id != row_id);
                                            let values = body_kvs.iter()
                                                .filter(|(k, _)| !k.is_empty())
                                                .map(|(k, v)| (k.clone(), v.clone()))
                                                .collect();
                                            table.rows.push(core::models::FakeDbRow { id: row_id, values });
                                        }
                                    }

                                    // READ ALL → replace entire table with server response
                                    "GET" if s == 200 => {
                                        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&result.body) {
                                            if let Some(table) = state.project.get_fake_table_mut(&model_id) {
                                                table.rows.clear();
                                                for item in &arr {
                                                    if let Some(id) = item.get("id").and_then(|v| v.as_u64()) {
                                                        let values: Vec<(String, String)> = item.as_object()
                                                            .map(|obj| obj.iter()
                                                                .filter(|(k, _)| k.as_str() != "id")
                                                                .map(|(k, v)| (k.clone(), v.to_string().trim_matches('"').to_string()))
                                                                .collect())
                                                            .unwrap_or_default();
                                                        table.rows.push(core::models::FakeDbRow { id, values });
                                                        if id >= table.next_id { table.next_id = id + 1; }
                                                    }
                                                }
                                            }
                                        } else if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&result.body) {
                                            // READ ONE — update or insert single row
                                            if let Some(id) = obj.get("id").and_then(|v| v.as_u64()) {
                                                if let Some(table) = state.project.get_fake_table_mut(&model_id) {
                                                    table.rows.retain(|r| r.id != id);
                                                    let values: Vec<(String, String)> = obj.as_object()
                                                        .map(|o| o.iter()
                                                            .filter(|(k, _)| k.as_str() != "id")
                                                            .map(|(k, v)| (k.clone(), v.to_string().trim_matches('"').to_string()))
                                                            .collect())
                                                        .unwrap_or_default();
                                                    table.rows.push(core::models::FakeDbRow { id, values });
                                                    if id >= table.next_id { table.next_id = id + 1; }
                                                }
                                            }
                                        }
                                    }

                                    // UPDATE — patch row values; use id from path or response
                                    "PUT" | "PATCH" if s == 200 => {
                                        // Prefer id from JSON response; fall back to path param
                                        let real_id: Option<u64> = serde_json::from_str::<serde_json::Value>(&result.body)
                                            .ok()
                                            .and_then(|v| v.get("id").and_then(|id| id.as_u64()))
                                            .or_else(|| path_kvs.first().and_then(|(_, v)| v.parse().ok()));

                                        if let Some(row_id) = real_id {
                                            if let Some(table) = state.project.get_fake_table_mut(&model_id) {
                                                if let Some(row) = table.rows.iter_mut().find(|r| r.id == row_id) {
                                                    for (k, v) in &body_kvs {
                                                        if let Some(cell) = row.values.iter_mut().find(|(rk, _)| rk == k) {
                                                            cell.1 = v.clone();
                                                        } else if !k.is_empty() {
                                                            row.values.push((k.clone(), v.clone()));
                                                        }
                                                    }
                                                } else {
                                                    // Row not in fake DB yet — add it
                                                    let values = body_kvs.iter()
                                                        .filter(|(k, _)| !k.is_empty())
                                                        .map(|(k, v)| (k.clone(), v.clone()))
                                                        .collect();
                                                    table.rows.push(core::models::FakeDbRow { id: row_id, values });
                                                }
                                            }
                                        }
                                    }

                                    // DELETE — remove row
                                    "DELETE" if s == 200 || s == 204 => {
                                        let row_id: Option<u64> = path_kvs.first()
                                            .and_then(|(_, v)| v.parse().ok());
                                        if let Some(id) = row_id {
                                            if let Some(table) = state.project.get_fake_table_mut(&model_id) {
                                                table.rows.retain(|r| r.id != id);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        state.tester.response = Some(result);
                    }
                    AppAction::Export { path } => {
                        match export_project(&state.project, &path) {
                            Ok(files) => {
                                let msg = format!("✓ Exported {} files to {}", files.len(), path);
                                state.notify(msg.clone());
                                state.server_logs.push(format!("[export] {}", msg));
                            }
                            Err(e) => {
                                state.notify(format!("Export failed: {}", e));
                                state.server_logs.push(format!("[export] ERROR: {}", e));
                            }
                        }
                    }
                    AppAction::None => {}
                }
            }
        }
    }

    // ── Cleanup ──────────────────────────────────────────────────────────────
    if let Some(ref mut child) = server_child {
        let _ = child.kill();
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    // ── Auto-save on exit ────────────────────────────────────────────────────
    match save_project(&state.project) {
        Ok(path) => {
            println!("\n✓ Project '{}' saved to {}", state.project.name, path.display());
            println!("  Next time: backforge  (pick from list)");
            println!("  Or:        backforge \"{}\"", state.project.name);
        }
        Err(e) => println!("\n⚠ Could not save project: {}", e),
    }

    Ok(())
}

// ─── HTTP Request Firing ──────────────────────────────────────────────────────

fn fire_http_request(req: FireRequest) -> TesterResponse {
    use serde_json::{Map, Value};
    let start = std::time::Instant::now();

    let body_schema: std::collections::HashMap<String, String> = req
        .body_schema
        .into_iter()
        .collect();

    let mut body_map: Map<String, Value> = Map::new();
    for (k, raw) in req.body_kvs {
        if k.is_empty() || raw.is_empty() {
            continue;
        }
        let ty = body_schema.get(&k).map(String::as_str).unwrap_or("str");
        let value = match coerce_typed_json_value(&raw, ty) {
            Ok(v) => v,
            Err(msg) => {
                return TesterResponse {
                    status: 0,
                    body: format!("Invalid value for '{}' as {}: {}", k, ty, msg),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                }
            }
        };
        body_map.insert(k, value);
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return TesterResponse {
            status: 0,
            body: format!("Failed to build HTTP client: {}", e),
            elapsed_ms: 0,
        },
    };

    let method_upper = req.method.to_uppercase();
    let builder = match method_upper.as_str() {
        "GET"    => client.get(&req.url),
        "POST"   => client.post(&req.url),
        "PUT"    => client.put(&req.url),
        "PATCH"  => client.patch(&req.url),
        "DELETE" => client.delete(&req.url),
        other    => return TesterResponse {
            status: 0,
            body: format!("Unsupported method: {}", other),
            elapsed_ms: 0,
        },
    };

    let builder = if matches!(method_upper.as_str(), "POST" | "PUT" | "PATCH") && !body_map.is_empty() {
        builder.json(&body_map)
    } else {
        builder
    };

    let builder = if !req.auth_token.is_empty() {
        builder.bearer_auth(&req.auth_token)
    } else {
        builder
    };

    match builder.send() {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body = resp.text().unwrap_or_else(|_| "(unreadable body)".to_string());
            let body = serde_json::from_str::<serde_json::Value>(&body)
                .map(|v| serde_json::to_string_pretty(&v).unwrap_or(body.clone()))
                .unwrap_or(body);
            TesterResponse { status, body, elapsed_ms: start.elapsed().as_millis() as u64 }
        }
        Err(e) => TesterResponse {
            status: 0,
            body: format!("Request failed: {}\n\nIs server running? Go to Server Runner → [s]", e),
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
    }
}

fn coerce_typed_json_value(raw: &str, ty: &str) -> Result<serde_json::Value, String> {
    use serde_json::{Number, Value};

    match ty.to_lowercase().as_str() {
        "int" | "integer" => raw
            .parse::<i64>()
            .map(|n| Value::Number(Number::from(n)))
            .map_err(|_| "expected an integer".to_string()),
        "float" => {
            let n = raw
                .parse::<f64>()
                .map_err(|_| "expected a float".to_string())?;
            Number::from_f64(n)
                .map(Value::Number)
                .ok_or_else(|| "float is not finite".to_string())
        }
        "bool" | "boolean" => raw
            .parse::<bool>()
            .map(Value::Bool)
            .map_err(|_| "expected true or false".to_string()),
        "dict" | "json" => serde_json::from_str::<Value>(raw)
            .map_err(|_| "expected valid JSON object/value".to_string()),
        _ => Ok(Value::String(raw.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::coerce_typed_json_value;
    use serde_json::json;

    #[test]
    fn coercion_integer_boolean_and_json() {
        assert_eq!(coerce_typed_json_value("42", "int").unwrap(), json!(42));
        assert_eq!(coerce_typed_json_value("true", "bool").unwrap(), json!(true));
        assert_eq!(
            coerce_typed_json_value("{\"a\":1}", "json").unwrap(),
            json!({"a": 1})
        );
    }

    #[test]
    fn coercion_rejects_invalid_integer() {
        let err = coerce_typed_json_value("x", "int").unwrap_err();
        assert!(err.contains("integer"));
    }
}

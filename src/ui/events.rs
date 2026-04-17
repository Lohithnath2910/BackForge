use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use crate::ui::state::{AppState, Screen, Modal, FieldForm, EndpointForm, ModelForm, ServerStatus};
use crate::core::models::*;

pub struct FireRequest {
    pub method: String,
    pub url: String,
    pub body_kvs: Vec<(String, String)>,
    pub body_schema: Vec<(String, String)>,
    pub path_kvs: Vec<(String, String)>,
    pub auth_token: String,
}

pub enum AppAction {
    None,
    Quit,
    StartServer,
    StopServer,
    TestEndpoint { endpoint_id: String },
    SendRequest(FireRequest),
    Export { path: String },
}

pub fn handle_key(state: &mut AppState, key: KeyEvent) -> AppAction {
    // Global shortcuts
    if key.modifiers == KeyModifiers::CONTROL {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => return AppAction::Quit,
            KeyCode::Char('h') => {
                state.screen = Screen::Help;
                return AppAction::None;
            }
            _ => {}
        }
    }

    // Dismiss notification
    if state.notification.is_some() {
        if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
            state.notification = None;
        }
    }

    // Modal handling takes priority
    match &state.modal.clone() {
        Modal::None => {}
        Modal::NewModel => return handle_new_model_modal(state, key),
        Modal::NewField => return handle_new_field_modal(state, key),
        Modal::NewEndpoint => return handle_new_endpoint_modal(state, key),
        Modal::ConfirmDelete(id) => {
            let id = id.clone();
            return handle_confirm_delete(state, key, &id);
        }
        Modal::AuthEnable => return handle_auth_modal(state, key),
    }

    // Screen-specific handling
    match state.screen.clone() {
        Screen::Home => handle_home(state, key),
        Screen::ModelList => handle_model_list(state, key),
        Screen::ModelEditor => handle_model_editor(state, key),
        Screen::EndpointList => handle_endpoint_list(state, key),
        Screen::EndpointEditor => handle_endpoint_editor(state, key),
        Screen::AuthSetup => handle_auth_setup(state, key),
        Screen::FakeDbViewer => handle_fake_db(state, key),
        Screen::ServerRunner => handle_server_runner(state, key),
        Screen::EndpointTester => handle_endpoint_tester(state, key),
        Screen::ExportPanel => handle_export(state, key),
        Screen::Help => handle_help(state, key),
    }
}

fn handle_home(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Char('m') | KeyCode::Char('M') => state.screen = Screen::ModelList,
        KeyCode::Char('e') | KeyCode::Char('E') => state.screen = Screen::EndpointList,
        KeyCode::Char('a') | KeyCode::Char('A') => state.screen = Screen::AuthSetup,
        KeyCode::Char('d') | KeyCode::Char('D') => state.screen = Screen::FakeDbViewer,
        KeyCode::Char('s') | KeyCode::Char('S') => state.screen = Screen::ServerRunner,
        KeyCode::Char('x') | KeyCode::Char('X') => state.screen = Screen::ExportPanel,
        KeyCode::Char('h') | KeyCode::Char('?') => state.screen = Screen::Help,
        KeyCode::Char('q') => return AppAction::Quit,
        _ => {}
    }
    AppAction::None
}

fn handle_model_list(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state.screen = Screen::Home,
        KeyCode::Char('n') => {
            state.model_form = Default::default();
            state.modal = Modal::NewModel;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.selected_model_idx > 0 {
                state.selected_model_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !state.project.models.is_empty()
                && state.selected_model_idx < state.project.models.len() - 1
            {
                state.selected_model_idx += 1;
            }
        }
        KeyCode::Enter => {
            if !state.project.models.is_empty() {
                state.selected_field_idx = 0;
                state.screen = Screen::ModelEditor;
            }
        }
        KeyCode::Char('d') => {
            if let Some(m) = state.project.models.get(state.selected_model_idx) {
                let id = m.id.clone();
                state.modal = Modal::ConfirmDelete(id);
            }
        }
        KeyCode::Char('r') => {
            if let Some(m) = state.project.models.get(state.selected_model_idx) {
                state.model_form.name = m.name.clone();
                state.editing_model_id = Some(m.id.clone());
                state.modal = Modal::NewModel;
            }
        }
        KeyCode::Char('v') => state.show_er_diagram = !state.show_er_diagram,
        _ => {}
    }
    AppAction::None
}

fn handle_model_editor(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.screen = Screen::ModelList;
        }
        KeyCode::Char('n') => {
            state.field_form = Default::default();
            state.modal = Modal::NewField;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.selected_field_idx > 0 {
                state.selected_field_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(model) = state.project.models.get(state.selected_model_idx) {
                if state.selected_field_idx < model.fields.len().saturating_sub(1) {
                    state.selected_field_idx += 1;
                }
            }
        }
        KeyCode::Char('d') => {
            if let Some(model) = state.project.models.get_mut(state.selected_model_idx) {
                let idx = state.selected_field_idx;
                if idx < model.fields.len() && !model.fields[idx].primary_key {
                    model.fields.remove(idx);
                    if state.selected_field_idx > 0 {
                        state.selected_field_idx -= 1;
                    }
                }
            }
        }
        KeyCode::Char('u') => {
            // Toggle nullable on selected field
            if let Some(model) = state.project.models.get_mut(state.selected_model_idx) {
                let idx = state.selected_field_idx;
                if idx < model.fields.len() && !model.fields[idx].primary_key {
                    model.fields[idx].nullable = !model.fields[idx].nullable;
                }
            }
        }
        _ => {}
    }
    AppAction::None
}

fn handle_endpoint_list(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state.screen = Screen::Home,
        KeyCode::Char('n') => {
            state.endpoint_form = Default::default();
            state.modal = Modal::NewEndpoint;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.selected_endpoint_idx > 0 {
                state.selected_endpoint_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !state.project.endpoints.is_empty()
                && state.selected_endpoint_idx < state.project.endpoints.len() - 1
            {
                state.selected_endpoint_idx += 1;
            }
        }
        KeyCode::Enter => {
            if !state.project.endpoints.is_empty() {
                state.screen = Screen::EndpointEditor;
            }
        }
        KeyCode::Char('t') => {
            if let Some(ep) = state.project.endpoints.get(state.selected_endpoint_idx) {
                let ep_id = ep.id.clone();
                return AppAction::TestEndpoint { endpoint_id: ep_id };
            }
        }
        KeyCode::Char('d') => {
            if let Some(ep) = state.project.endpoints.get(state.selected_endpoint_idx) {
                let id = ep.id.clone();
                state.modal = Modal::ConfirmDelete(id);
            }
        }
        _ => {}
    }
    AppAction::None
}

fn handle_endpoint_editor(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state.screen = Screen::EndpointList,
        KeyCode::Char('e') => {
            // Pre-fill EndpointForm with current endpoint values
            if let Some(ep) = state.project.endpoints.get(state.selected_endpoint_idx) {
                let method_idx = HttpMethod::variants()
                    .iter().position(|m| m.as_str() == ep.method.as_str())
                    .unwrap_or(0);
                let crud_idx = [
                    CrudOp::Create, CrudOp::ReadOne, CrudOp::ReadAll,
                    CrudOp::Update, CrudOp::Delete, CrudOp::Custom,
                ].iter().position(|c| c.as_str() == ep.crud_op.as_str())
                    .unwrap_or(0);
                let linked_model_index = if let Some(ref mid) = ep.linked_model {
                    state.project.models.iter().position(|m| &m.id == mid)
                        .map(|i| i + 1).unwrap_or(0)
                } else { 0 };

                state.endpoint_form = EndpointForm {
                    path: ep.path.clone(),
                    method_index: method_idx,
                    crud_op_index: crud_idx,
                    description: ep.description.clone(),
                    requires_auth: ep.requires_auth,
                    linked_model_index,
                    body_params: ep.body_params.iter().map(|(k, _)| k.clone()).collect(),
                    focused_field: 0,
                    field_picker_idx: 0,
                };
                state.editing_endpoint_id = Some(ep.id.clone());
                state.modal = Modal::NewEndpoint;
            }
        }
        KeyCode::Char('a') => {
            if let Some(ep) = state.project.endpoints.get_mut(state.selected_endpoint_idx) {
                ep.requires_auth = !ep.requires_auth;
            }
        }
        _ => {}
    }
    AppAction::None
}

fn handle_new_model_modal(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc => {
            state.modal = Modal::None;
            state.model_form = ModelForm::default();
            state.editing_model_id = None;
        }
        KeyCode::Enter => {
            let name = state.model_form.name.trim().to_string();
            if name.is_empty() { return AppAction::None; }

            if let Some(ref edit_id) = state.editing_model_id.clone() {
                // Rename: check no OTHER model has this name
                let dup = state.project.models.iter().any(|m| m.name == name && &m.id != edit_id);
                if dup {
                    state.notify(format!("'{}' already exists!", name));
                } else if let Some(m) = state.project.models.iter_mut().find(|m| &m.id == edit_id) {
                    m.name = name.clone();
                    state.notify(format!("Model renamed to '{}'!", name));
                }
            } else {
                // Create new
                let dup = state.project.models.iter().any(|m| m.name == name);
                if dup {
                    state.notify(format!("Model '{}' already exists!", name));
                } else {
                    let model = Model::new(name.clone());
                    let model_id = model.id.clone();
                    state.project.models.push(model);
                    state.project.fake_db.push(FakeDbTable::new(model_id));
                    state.notify(format!("Model '{}' created!", name));
                }
            }
            state.modal = Modal::None;
            state.model_form = ModelForm::default();
            state.editing_model_id = None;
        }
        KeyCode::Char(c) => state.model_form.name.push(c),
        KeyCode::Backspace => { state.model_form.name.pop(); }
        _ => {}
    }
    AppAction::None
}

fn handle_new_field_modal(state: &mut AppState, key: KeyEvent) -> AppAction {
    let total_fields = 5; // name, type, nullable, unique, pk
    match key.code {
        KeyCode::Esc => state.modal = Modal::None,
        KeyCode::Tab | KeyCode::Down => {
            state.field_form.focused_field = (state.field_form.focused_field + 1) % total_fields;
        }
        KeyCode::BackTab | KeyCode::Up => {
            if state.field_form.focused_field == 0 {
                state.field_form.focused_field = total_fields - 1;
            } else {
                state.field_form.focused_field -= 1;
            }
        }
        KeyCode::Enter => {
            if state.field_form.focused_field == total_fields - 1
                || key.modifiers == crossterm::event::KeyModifiers::CONTROL
            {
                // Commit the field
                let name = state.field_form.name.trim().to_string();
                if name.is_empty() {
                    return AppAction::None;
                }
                let types = DataType::variants();
                let data_type = types[state.field_form.data_type_index].clone();
                let field = ModelField {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.clone(),
                    data_type,
                    nullable: state.field_form.nullable,
                    unique: state.field_form.unique,
                    primary_key: state.field_form.primary_key,
                    default_value: None,
                };
                if let Some(model) = state.project.models.get_mut(state.selected_model_idx) {
                    model.fields.push(field);
                }
                state.notify(format!("Field '{}' added!", name));
                state.modal = Modal::None;
                state.field_form = FieldForm::default();
            }
        }
        KeyCode::Left | KeyCode::Right => {
            let ff = state.field_form.focused_field;
            if ff == 1 {
                let types_len = DataType::variants().len();
                if key.code == KeyCode::Left {
                    if state.field_form.data_type_index == 0 {
                        state.field_form.data_type_index = types_len - 1;
                    } else {
                        state.field_form.data_type_index -= 1;
                    }
                } else {
                    state.field_form.data_type_index = (state.field_form.data_type_index + 1) % types_len;
                }
            } else if ff == 2 {
                state.field_form.nullable = !state.field_form.nullable;
            } else if ff == 3 {
                state.field_form.unique = !state.field_form.unique;
            } else if ff == 4 {
                state.field_form.primary_key = !state.field_form.primary_key;
            }
        }
        KeyCode::Char(' ') => {
            let ff = state.field_form.focused_field;
            if ff == 2 {
                state.field_form.nullable = !state.field_form.nullable;
            } else if ff == 3 {
                state.field_form.unique = !state.field_form.unique;
            } else if ff == 4 {
                state.field_form.primary_key = !state.field_form.primary_key;
            } else if ff == 0 {
                state.field_form.name.push(' ');
            }
        }
        KeyCode::Char(c) => {
            if state.field_form.focused_field == 0 {
                state.field_form.name.push(c);
            }
        }
        KeyCode::Backspace => {
            if state.field_form.focused_field == 0 {
                state.field_form.name.pop();
            }
        }
        _ => {}
    }
    AppAction::None
}

fn handle_new_endpoint_modal(state: &mut AppState, key: KeyEvent) -> AppAction {
    // Fields: 0=path, 1=method, 2=crud(optional), 3=auth, 4=model, 5=fields row, 6=note
    let total_fields = 7;
    match key.code {
        KeyCode::Esc => {
            state.modal = Modal::None;
            state.editing_endpoint_id = None;
            state.endpoint_form = EndpointForm::default();
        }
        KeyCode::Tab | KeyCode::Down => {
            state.endpoint_form.focused_field =
                (state.endpoint_form.focused_field + 1) % total_fields;
        }
        KeyCode::BackTab | KeyCode::Up => {
            if state.endpoint_form.focused_field == 0 {
                state.endpoint_form.focused_field = total_fields - 1;
            } else {
                state.endpoint_form.focused_field -= 1;
            }
        }
        KeyCode::Enter => {
            if state.endpoint_form.focused_field == total_fields - 1 {
                // Last field (Note) — commit
                commit_endpoint(state);
            } else if state.endpoint_form.focused_field == 5 {
                // On the field-picker row: add currently highlighted model field
                add_picked_field(state);
            } else {
                commit_endpoint(state);
            }
        }
        KeyCode::Left  => handle_endpoint_form_left(state),
        KeyCode::Right => handle_endpoint_form_right(state),
        KeyCode::Char(c) => {
            match state.endpoint_form.focused_field {
                0 => {
                    state.endpoint_form.path.push(c);
                    // Auto-update CRUD suggestion when path changes
                    state.endpoint_form.crud_op_index =
                        EndpointForm::auto_crud_from_method_path(
                            state.endpoint_form.method_index,
                            &state.endpoint_form.path.clone(),
                        );
                }
                5 => {
                    // Space = add picked field; letters = cycle to matching field
                    if c == ' ' {
                        add_picked_field(state);
                    }
                }
                6 => state.endpoint_form.description.push(c),
                _ => {}
            }
        }
        KeyCode::Backspace => {
            match state.endpoint_form.focused_field {
                0 => { state.endpoint_form.path.pop(); }
                5 => {
                    // Backspace removes last added field
                    state.endpoint_form.body_params.pop();
                }
                6 => { state.endpoint_form.description.pop(); }
                _ => {}
            }
        }
        KeyCode::Delete => {
            if state.endpoint_form.focused_field == 5 {
                state.endpoint_form.body_params.pop();
            }
        }
        _ => {}
    }
    AppAction::None
}

/// Add the currently highlighted model field to body_params
fn add_picked_field(state: &mut AppState) {
    // Get fields from the linked model
    let model_fields: Vec<String> = if state.endpoint_form.linked_model_index > 0 {
        state.project.models
            .get(state.endpoint_form.linked_model_index - 1)
            .map(|m| m.fields.iter().filter(|f| !f.primary_key).map(|f| f.name.clone()).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if model_fields.is_empty() { return; }

    let idx = state.endpoint_form.field_picker_idx % model_fields.len();
    let field_name = model_fields[idx].clone();

    // Toggle: if already added, remove it; else add it
    if let Some(pos) = state.endpoint_form.body_params.iter().position(|k| k == &field_name) {
        state.endpoint_form.body_params.remove(pos);
    } else {
        state.endpoint_form.body_params.push(field_name);
    }
}

fn commit_endpoint(state: &mut AppState) {
    let path = state.endpoint_form.path.trim().to_string();
    if path.is_empty() { return; }

    let methods = HttpMethod::variants();
    let method = methods[state.endpoint_form.method_index].clone();
    let crud_ops = vec![
        CrudOp::Create, CrudOp::ReadOne, CrudOp::ReadAll,
        CrudOp::Update, CrudOp::Delete, CrudOp::Custom,
    ];
    let crud_op = crud_ops[state.endpoint_form.crud_op_index].clone();

    let linked_model = if state.endpoint_form.linked_model_index == 0 {
        None
    } else {
        state.project.models
            .get(state.endpoint_form.linked_model_index - 1)
            .map(|m| m.id.clone())
    };

    let method_uses_body = matches!(method, HttpMethod::POST | HttpMethod::PUT | HttpMethod::PATCH);
    let selected_fields = &state.endpoint_form.body_params;

    let typed_fields: Vec<(String, String)> = if let Some(ref model_id) = linked_model {
        state.project
            .models
            .iter()
            .find(|m| &m.id == model_id)
            .map(|m| {
                selected_fields
                    .iter()
                    .map(|name| {
                        let ty = m
                            .fields
                            .iter()
                            .find(|f| &f.name == name)
                            .map(|f| f.data_type.to_python_type().to_string())
                            .unwrap_or_else(|| "str".to_string());
                        (name.clone(), ty)
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                selected_fields
                    .iter()
                    .map(|name| (name.clone(), "str".to_string()))
                    .collect()
            })
    } else {
        selected_fields
            .iter()
            .map(|name| (name.clone(), "str".to_string()))
            .collect()
    };

    // If editing an existing endpoint, preserve its id and replace in-place
    let ep_id = state.editing_endpoint_id.clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let ep = Endpoint {
        id: ep_id.clone(),
        path: path.clone(),
        method,
        crud_op,
        linked_model,
        description: state.endpoint_form.description.clone(),
        requires_auth: state.endpoint_form.requires_auth,
        path_params: extract_path_params(&path),
        query_params: if method_uses_body {
            Vec::new()
        } else {
            typed_fields.iter().map(|(name, _)| name.clone()).collect()
        },
        body_params: if method_uses_body { typed_fields } else { Vec::new() },
        tags: Vec::new(),
    };

    if let Some(ref edit_id) = state.editing_endpoint_id.clone() {
        // Replace existing
        if let Some(existing) = state.project.endpoints.iter_mut().find(|e| &e.id == edit_id) {
            *existing = ep;
        }
        state.notify(format!("Endpoint '{}' updated!", path));
    } else {
        state.project.endpoints.push(ep);
        state.notify(format!("Endpoint '{}' created!", path));
    }

    state.modal = Modal::None;
    state.endpoint_form = EndpointForm::default();
    state.editing_endpoint_id = None;
}

fn extract_path_params(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut in_param = false;
    let mut current = String::new();
    for c in path.chars() {
        if c == '{' {
            in_param = true;
        } else if c == '}' {
            if in_param && !current.is_empty() {
                params.push(current.clone());
                current.clear();
            }
            in_param = false;
        } else if in_param {
            current.push(c);
        }
    }
    params
}

fn handle_endpoint_form_left(state: &mut AppState) {
    match state.endpoint_form.focused_field {
        1 => {
            let len = HttpMethod::variants().len();
            if state.endpoint_form.method_index == 0 {
                state.endpoint_form.method_index = len - 1;
            } else {
                state.endpoint_form.method_index -= 1;
            }
            // Auto-update CRUD suggestion
            state.endpoint_form.crud_op_index = EndpointForm::auto_crud_from_method_path(
                state.endpoint_form.method_index, &state.endpoint_form.path.clone());
        }
        2 => {
            if state.endpoint_form.crud_op_index == 0 {
                state.endpoint_form.crud_op_index = 5;
            } else {
                state.endpoint_form.crud_op_index -= 1;
            }
        }
        3 => state.endpoint_form.requires_auth = !state.endpoint_form.requires_auth,
        4 => {
            let len = state.project.models.len() + 1;
            if state.endpoint_form.linked_model_index == 0 {
                state.endpoint_form.linked_model_index = len - 1;
            } else {
                state.endpoint_form.linked_model_index -= 1;
            }
        }
        5 => {
            // Cycle picker left through model fields
            let count = linked_model_field_count(state);
            if count > 0 {
                if state.endpoint_form.field_picker_idx == 0 {
                    state.endpoint_form.field_picker_idx = count - 1;
                } else {
                    state.endpoint_form.field_picker_idx -= 1;
                }
            }
        }
        _ => {}
    }
}

fn handle_endpoint_form_right(state: &mut AppState) {
    match state.endpoint_form.focused_field {
        1 => {
            state.endpoint_form.method_index =
                (state.endpoint_form.method_index + 1) % HttpMethod::variants().len();
            state.endpoint_form.crud_op_index = EndpointForm::auto_crud_from_method_path(
                state.endpoint_form.method_index, &state.endpoint_form.path.clone());
        }
        2 => {
            state.endpoint_form.crud_op_index = (state.endpoint_form.crud_op_index + 1) % 6;
        }
        3 => state.endpoint_form.requires_auth = !state.endpoint_form.requires_auth,
        4 => {
            let len = state.project.models.len() + 1;
            state.endpoint_form.linked_model_index =
                (state.endpoint_form.linked_model_index + 1) % len.max(1);
        }
        5 => {
            // Cycle picker right through model fields
            let count = linked_model_field_count(state);
            if count > 0 {
                state.endpoint_form.field_picker_idx =
                    (state.endpoint_form.field_picker_idx + 1) % count;
            }
        }
        _ => {}
    }
}

fn linked_model_field_count(state: &AppState) -> usize {
    if state.endpoint_form.linked_model_index == 0 { return 0; }
    state.project.models
        .get(state.endpoint_form.linked_model_index - 1)
        .map(|m| m.fields.iter().filter(|f| !f.primary_key).count())
        .unwrap_or(0)
}

fn handle_confirm_delete(state: &mut AppState, key: KeyEvent, id: &str) -> AppAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            // Try model first, then endpoint
            let before = state.project.models.len();
            state.project.models.retain(|m| m.id != id);
            state.project.fake_db.retain(|t| t.model_id != id);
            if state.project.models.len() < before {
                if state.selected_model_idx > 0 {
                    state.selected_model_idx -= 1;
                }
                state.notify("Model deleted.".to_string());
            } else {
                state.project.endpoints.retain(|e| e.id != id);
                if state.selected_endpoint_idx > 0 {
                    state.selected_endpoint_idx -= 1;
                }
                state.notify("Endpoint deleted.".to_string());
            }
            state.modal = Modal::None;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            state.modal = Modal::None;
        }
        _ => {}
    }
    AppAction::None
}

fn handle_auth_modal(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc => state.modal = Modal::None,
        KeyCode::Enter => {
            state.project.auth_config.enabled = true;
            let strategies = vec![AuthStrategy::JWT, AuthStrategy::Session, AuthStrategy::APIKey];
            state.project.auth_config.strategy = strategies[state.auth_strategy_idx].clone();
            // Add auth endpoints automatically
            add_auth_endpoints(state);
            state.notify("Auth enabled! Endpoints auto-generated.".to_string());
            state.modal = Modal::None;
        }
        KeyCode::Left | KeyCode::Right => {
            let len = 3;
            if key.code == KeyCode::Left {
                if state.auth_strategy_idx == 0 {
                    state.auth_strategy_idx = len - 1;
                } else {
                    state.auth_strategy_idx -= 1;
                }
            } else {
                state.auth_strategy_idx = (state.auth_strategy_idx + 1) % len;
            }
        }
        _ => {}
    }
    AppAction::None
}

fn add_auth_endpoints(state: &mut AppState) {
    // Remove old auth endpoints if re-enabling
    state.project.endpoints.retain(|e| !e.tags.contains(&"auth".to_string()));

    let auth_endpoints = vec![
        ("/auth/register", HttpMethod::POST, "Register new user"),
        ("/auth/login", HttpMethod::POST, "Login and get token"),
        ("/auth/logout", HttpMethod::POST, "Logout / revoke token"),
        ("/auth/me", HttpMethod::GET, "Get current user profile"),
        ("/auth/refresh", HttpMethod::POST, "Refresh access token"),
    ];

    for (path, method, desc) in auth_endpoints {
        let mut ep = Endpoint::new(path.to_string(), method);
        ep.description = desc.to_string();
        ep.tags = vec!["auth".to_string()];
        state.project.endpoints.push(ep);
    }
}

fn handle_auth_setup(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state.screen = Screen::Home,
        KeyCode::Char('e') => {
            if !state.project.auth_config.enabled {
                state.modal = Modal::AuthEnable;
            }
        }
        KeyCode::Char('d') => {
            state.project.auth_config.enabled = false;
            state.project.endpoints.retain(|e| !e.tags.contains(&"auth".to_string()));
            state.notify("Auth disabled.".to_string());
        }
        KeyCode::Left | KeyCode::Right => {
            if state.project.auth_config.enabled {
                let len = 3;
                if key.code == KeyCode::Left {
                    if state.auth_strategy_idx == 0 {
                        state.auth_strategy_idx = len - 1;
                    } else {
                        state.auth_strategy_idx -= 1;
                    }
                } else {
                    state.auth_strategy_idx = (state.auth_strategy_idx + 1) % len;
                }
                let strategies = vec![AuthStrategy::JWT, AuthStrategy::Session, AuthStrategy::APIKey];
                state.project.auth_config.strategy = strategies[state.auth_strategy_idx].clone();
            }
        }
        _ => {}
    }
    AppAction::None
}

fn handle_fake_db(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state.screen = Screen::Home,
        KeyCode::Left | KeyCode::Char('h') => {
            if state.fake_db_model_idx > 0 {
                state.fake_db_model_idx -= 1;
                state.fake_db_scroll = 0;
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if !state.project.models.is_empty()
                && state.fake_db_model_idx < state.project.models.len() - 1
            {
                state.fake_db_model_idx += 1;
                state.fake_db_scroll = 0;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.fake_db_scroll > 0 {
                state.fake_db_scroll -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.fake_db_scroll += 1;
        }
        KeyCode::Char('c') => {
            // Clear table
            if let Some(model) = state.project.models.get(state.fake_db_model_idx) {
                let model_id = model.id.clone();
                if let Some(table) = state.project.get_fake_table_mut(&model_id) {
                    table.rows.clear();
                    table.next_id = 1;
                }
                state.notify("Table cleared.".to_string());
            }
        }
        _ => {}
    }
    AppAction::None
}

fn handle_server_runner(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state.screen = Screen::Home,
        KeyCode::Char('s') => {
            if state.server_status == ServerStatus::Stopped {
                return AppAction::StartServer;
            }
        }
        KeyCode::Char('x') => {
            if state.server_status != ServerStatus::Stopped {
                return AppAction::StopServer;
            }
        }
        KeyCode::Char('c') => {
            state.server_logs.clear();
        }
        _ => {}
    }
    AppAction::None
}

fn handle_export(state: &mut AppState, key: KeyEvent) -> AppAction {
    if state.export_path_editing {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                state.export_path_editing = false;
            }
            KeyCode::Char(c) => state.export_path.push(c),
            KeyCode::Backspace => { state.export_path.pop(); }
            _ => {}
        }
        return AppAction::None;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state.screen = Screen::Home,
        KeyCode::Char('e') => {
            state.export_path_editing = true;
        }
        KeyCode::Enter | KeyCode::Char('x') => {
            let path = state.export_path.clone();
            return AppAction::Export { path };
        }
        _ => {}
    }
    AppAction::None
}

fn handle_help(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state.screen = Screen::Home,
        KeyCode::Down | KeyCode::Char('j') => state.help_scroll += 1,
        KeyCode::Up | KeyCode::Char('k') => {
            if state.help_scroll > 0 {
                state.help_scroll -= 1;
            }
        }
        _ => {}
    }
    AppAction::None
}

fn handle_endpoint_tester(state: &mut AppState, key: KeyEvent) -> AppAction {
    let ep = match state.project.endpoints.get(state.tester.endpoint_idx) {
        Some(e) => e.clone(),
        None => { state.screen = Screen::EndpointList; return AppAction::None; }
    };

    if state.tester.picker_open {
        return handle_tester_picker(state, key, &ep);
    }

    let path_count = state.tester.path_kvs.len();
    let body_count = state.tester.body_kvs.len();
    let has_auth   = ep.requires_auth;
    let auth_focus = path_count + body_count;
    let send_focus = auth_focus + if has_auth { 1 } else { 0 };
    let total      = send_focus + 1;

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => { state.screen = Screen::EndpointList; }
        KeyCode::Tab | KeyCode::Down => {
            state.tester.focused = (state.tester.focused + 1) % total;
        }
        KeyCode::BackTab | KeyCode::Up => {
            if state.tester.focused == 0 { state.tester.focused = total - 1; }
            else { state.tester.focused -= 1; }
        }
        KeyCode::Enter => {
            if state.tester.focused == send_focus {
                return build_fire_request(state, &ep);
            }
        }
        KeyCode::Char('p') => {
            state.tester.picker_open = true;
            state.tester.picker_row_idx = 0;
        }
        KeyCode::Char(c) => {
            if key.modifiers == KeyModifiers::CONTROL && c == 'r' {
                return build_fire_request(state, &ep);
            }
            let f = state.tester.focused;
            if f < path_count {
                state.tester.path_kvs[f].1.push(c);
            } else if f < path_count + body_count {
                state.tester.body_kvs[f - path_count].1.push(c);
            } else if has_auth && f == auth_focus {
                state.tester.auth_token.push(c);
            }
        }
        KeyCode::Backspace => {
            let f = state.tester.focused;
            if f < path_count {
                state.tester.path_kvs[f].1.pop();
            } else if f < path_count + body_count {
                state.tester.body_kvs[f - path_count].1.pop();
            } else if has_auth && f == auth_focus {
                state.tester.auth_token.pop();
            }
        }
        _ => {}
    }
    AppAction::None
}

fn handle_tester_picker(state: &mut AppState, key: KeyEvent, ep: &Endpoint) -> AppAction {
    let rows: Vec<(u64, Vec<(String, String)>)> = ep.linked_model.as_ref()
        .and_then(|mid| state.project.get_fake_table(mid))
        .map(|t| t.rows.iter().map(|r| (r.id, r.values.clone())).collect())
        .unwrap_or_default();

    match key.code {
        KeyCode::Esc => { state.tester.picker_open = false; }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.tester.picker_row_idx > 0 { state.tester.picker_row_idx -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !rows.is_empty() && state.tester.picker_row_idx < rows.len() - 1 {
                state.tester.picker_row_idx += 1;
            }
        }
        KeyCode::Enter => {
            if let Some((row_id, row_values)) = rows.get(state.tester.picker_row_idx) {
                let f = state.tester.focused;
                let path_count = state.tester.path_kvs.len();
                let body_count = state.tester.body_kvs.len();
                if f < path_count {
                    state.tester.path_kvs[f].1 = row_id.to_string();
                } else if f < path_count + body_count {
                    let field_name = state.tester.body_kvs[f - path_count].0.clone();
                    let val = row_values.iter()
                        .find(|(k, _)| k == &field_name)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| row_id.to_string());
                    state.tester.body_kvs[f - path_count].1 = val;
                }
                state.tester.picker_open = false;
            }
        }
        _ => {}
    }
    AppAction::None
}

fn build_fire_request(state: &mut AppState, ep: &Endpoint) -> AppAction {
    let port = state.server_port;

    let mut url_path = ep.path.clone();
    for (k, v) in &state.tester.path_kvs {
        url_path = url_path.replace(&format!("{{{}}}", k), v);
    }

    let prefix = if let Some(model_id) = &ep.linked_model {
        state.project.models.iter()
            .find(|m| &m.id == model_id)
            .map(|m| format!("/{}", m.name.to_lowercase()
                .replace(' ', "_").replace('-', "_").replace('.', "_")))
            .unwrap_or_default()
    } else { String::new() };

    let url_path_clean = if url_path == "/" { String::new() } else { url_path };

    // GET / DELETE: append body_kvs as ?query=string
    let method_upper = ep.method.as_str().to_uppercase();
    let query_str = if matches!(method_upper.as_str(), "GET" | "DELETE") {
        encode_query_string(&state.tester.body_kvs)
    } else { String::new() };

    let url = format!("http://127.0.0.1:{}{}{}{}", port, prefix, url_path_clean, query_str);

    AppAction::SendRequest(FireRequest {
        method: ep.method.as_str().to_string(),
        url,
        body_kvs: state.tester.body_kvs.clone(),
        body_schema: ep.body_params.clone(),
        path_kvs: state.tester.path_kvs.clone(),
        auth_token: state.tester.auth_token.clone(),
    })
}

fn encode_query_string(kvs: &[(String, String)]) -> String {
    let params: Vec<String> = kvs
        .iter()
        .filter(|(k, v)| !k.is_empty() && !v.is_empty())
        .map(|(k, v)| {
            let key = percent_encode(k.as_bytes(), NON_ALPHANUMERIC).to_string();
            let value = percent_encode(v.as_bytes(), NON_ALPHANUMERIC).to_string();
            format!("{}={}", key, value)
        })
        .collect();

    if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    }
}

#[cfg(test)]
mod tests {
    use super::encode_query_string;

    #[test]
    fn query_string_is_url_encoded() {
        let kvs = vec![
            ("email".to_string(), "a+b@example.com".to_string()),
            ("name".to_string(), "John Doe".to_string()),
        ];
        let query = encode_query_string(&kvs);
        assert_eq!(query, "?email=a%2Bb%40example%2Ecom&name=John%20Doe");
    }

    #[test]
    fn query_string_skips_empty_values() {
        let kvs = vec![
            ("a".to_string(), "".to_string()),
            ("b".to_string(), "ok".to_string()),
        ];
        let query = encode_query_string(&kvs);
        assert_eq!(query, "?b=ok");
    }
}

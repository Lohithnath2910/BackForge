use crate::core::models::*;

pub struct NotificationToast {
    pub message: String,
    pub created_at: std::time::Instant,
    pub ttl: std::time::Duration,
}

// ─── App Screens ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    ModelList,
    ModelEditor,
    EndpointList,
    EndpointEditor,
    AuthSetup,
    FakeDbViewer,
    ServerRunner,
    EndpointTester,
    ExportPanel,
    Help,
}

// ─── Modal State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Modal {
    None,
    NewModel,
    NewField,
    NewEndpoint,
    ConfirmDelete(String),
    AuthEnable,
}

// ─── Form State ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ModelForm {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct FieldForm {
    pub name: String,
    pub data_type_index: usize,
    pub nullable: bool,
    pub unique: bool,
    pub primary_key: bool,
    pub focused_field: usize,
}
impl Default for FieldForm {
    fn default() -> Self {
        FieldForm { name: String::new(), data_type_index: 0, nullable: true, unique: false, primary_key: false, focused_field: 0 }
    }
}

/// Endpoint creation form.
///
/// BODY PARAMS meaning depends on HTTP method:
///   POST / PUT / PATCH → sent as JSON request body  (e.g. { "name": "...", "phone": "..." })
///   GET / DELETE       → sent as query string filters (e.g. ?name=Lohit)
///                         OR use {id} in the path for single-record operations
///
/// CRUD OP is optional metadata — it controls what code gets generated on export.
/// BackForge auto-suggests it from Method+Path but you can override with ← →.
///
/// PATH PARAMS: put {name} in the Path field (e.g. /{id}) and BackForge
/// auto-detects them and shows them as separate inputs in the tester.
#[derive(Debug, Clone)]
pub struct EndpointForm {
    pub path: String,
    pub method_index: usize,
    pub crud_op_index: usize,
    pub description: String,
    pub requires_auth: bool,
    pub linked_model_index: usize,
    pub body_params: Vec<String>,     // field names (picked from model or typed)
    pub focused_field: usize,
    /// Which field picker row is active (for cycling through model fields)
    pub field_picker_idx: usize,
}

impl Default for EndpointForm {
    fn default() -> Self {
        EndpointForm {
            path: "/".to_string(),
            method_index: 0,
            crud_op_index: 0,
            description: String::new(),
            requires_auth: false,
            linked_model_index: 0,
            body_params: Vec::new(),
            focused_field: 0,
            field_picker_idx: 0,
        }
    }
}

impl EndpointForm {
    /// Auto-suggest CRUD op index based on method and path
    pub fn auto_crud_from_method_path(method_idx: usize, path: &str) -> usize {
        let methods = HttpMethod::variants();
        let method = &methods[method_idx.min(methods.len() - 1)];
        let has_id_param = path.contains('{');
        match method {
            HttpMethod::POST    => 0, // Create
            HttpMethod::GET     => if has_id_param { 1 } else { 2 }, // ReadOne / ReadAll
            HttpMethod::PUT     => 3, // Update
            HttpMethod::PATCH   => 3, // Update
            HttpMethod::DELETE  => 4, // Delete
            HttpMethod::WebSocket => 5, // Custom
        }
    }

    /// Whether this method uses a request body (vs query params)
    pub fn method_uses_body(method_idx: usize) -> bool {
        let methods = HttpMethod::variants();
        matches!(methods.get(method_idx), Some(HttpMethod::POST) | Some(HttpMethod::PUT) | Some(HttpMethod::PATCH))
    }

    /// Label for the params section based on method
    pub fn params_section_label(method_idx: usize) -> &'static str {
        if Self::method_uses_body(method_idx) {
            "REQUEST BODY FIELDS  (sent as JSON — pick from model or type name)"
        } else {
            "FILTER FIELDS  (optional — sent as ?field=value query params)"
        }
    }
}

// ─── Endpoint Tester State ────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct TesterState {
    pub endpoint_idx: usize,
    /// body / query fields: (field_name, value_being_typed)
    pub body_kvs: Vec<(String, String)>,
    /// path params: (param_name, value_being_typed)
    pub path_kvs: Vec<(String, String)>,
    pub auth_token: String,
    pub focused: usize,
    pub response: Option<TesterResponse>,
    pub loading: bool,
    /// For the picker: which fake DB row is highlighted (for path param picker)
    pub picker_row_idx: usize,
    /// Whether the picker overlay is open
    pub picker_open: bool,
}

#[derive(Debug, Clone)]
pub struct TesterResponse {
    pub status: u16,
    pub body: String,
    pub elapsed_ms: u64,
}

// ─── Server State ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running { port: u16 },
    Error(String),
}

// ─── App State ────────────────────────────────────────────────────────────────

pub struct AppState {
    pub project: Project,
    pub screen: Screen,
    pub modal: Modal,
    pub selected_model_idx: usize,
    pub selected_field_idx: usize,
    pub selected_endpoint_idx: usize,
    pub model_form: ModelForm,
    pub field_form: FieldForm,
    pub endpoint_form: EndpointForm,
    pub tester: TesterState,
    pub server_status: ServerStatus,
    pub server_logs: Vec<String>,
    pub export_path: String,
    pub export_path_editing: bool,
    pub notification: Option<NotificationToast>,
    pub auth_strategy_idx: usize,
    pub fake_db_model_idx: usize,
    pub fake_db_scroll: usize,
    pub show_er_diagram: bool,
    pub help_scroll: usize,
    pub server_port: u16,
    pub editing_endpoint_id: Option<String>,
    pub editing_model_id: Option<String>,
}

impl AppState {
    pub fn new(project_name: String) -> Self {
        AppState {
            project: Project::new(project_name),
            screen: Screen::Home,
            modal: Modal::None,
            selected_model_idx: 0,
            selected_field_idx: 0,
            selected_endpoint_idx: 0,
            model_form: ModelForm::default(),
            field_form: FieldForm::default(),
            endpoint_form: EndpointForm::default(),
            tester: TesterState::default(),
            server_status: ServerStatus::Stopped,
            server_logs: Vec::new(),
            export_path: "./backforge_output".to_string(),
            export_path_editing: false,
            notification: None,
            auth_strategy_idx: 0,
            fake_db_model_idx: 0,
            fake_db_scroll: 0,
            show_er_diagram: true,
            help_scroll: 0,
            server_port: 8000,
            editing_endpoint_id: None,
            editing_model_id: None,
        }
    }

    pub fn notify(&mut self, msg: String) {
        let len = msg.len();
        let ttl_secs = if len <= 40 {
            2
        } else if len <= 100 {
            3
        } else {
            5
        };
        self.notification = Some(NotificationToast {
            message: msg,
            created_at: std::time::Instant::now(),
            ttl: std::time::Duration::from_secs(ttl_secs),
        });
    }

    pub fn tick_notification(&mut self) {
        if let Some(toast) = &self.notification {
            if toast.created_at.elapsed() >= toast.ttl {
                self.notification = None;
            }
        }
    }

    pub fn open_tester(&mut self, ep_idx: usize) {
        let mut t = TesterState {
            endpoint_idx: ep_idx,
            ..TesterState::default()
        };

        if let Some(ep) = self.project.endpoints.get(ep_idx) {
            let ep = ep.clone();
            for p in &ep.path_params {
                t.path_kvs.push((p.clone(), String::new()));
            }
            let uses_body = matches!(ep.method, HttpMethod::POST | HttpMethod::PUT | HttpMethod::PATCH);

            if uses_body {
                // Body fields: prefer explicit endpoint body schema, else pull model fields.
                if !ep.body_params.is_empty() {
                    for (k, _) in &ep.body_params {
                        t.body_kvs.push((k.clone(), String::new()));
                    }
                } else if let Some(model_id) = &ep.linked_model {
                    if let Some(model) = self.project.models.iter().find(|m| &m.id == model_id) {
                        for field in model.fields.iter().filter(|f| !f.primary_key) {
                            t.body_kvs.push((field.name.clone(), String::new()));
                        }
                    }
                }
            } else {
                // Query/filter fields: only use explicit endpoint query params.
                for k in &ep.query_params {
                    t.body_kvs.push((k.clone(), String::new()));
                }
            }
        }
        self.tester = t;
        self.screen = Screen::EndpointTester;
    }

    #[allow(dead_code)]
    pub fn selected_model(&self) -> Option<&Model> {
        self.project.models.get(self.selected_model_idx)
    }

    #[allow(dead_code)]
    pub fn selected_endpoint(&self) -> Option<&Endpoint> {
        self.project.endpoints.get(self.selected_endpoint_idx)
    }
}

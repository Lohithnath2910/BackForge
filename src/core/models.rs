use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Data Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum DataType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    UUID,
    Text,
    Json,
}

impl DataType {
    pub fn variants() -> Vec<DataType> {
        vec![
            DataType::String,
            DataType::Integer,
            DataType::Float,
            DataType::Boolean,
            DataType::DateTime,
            DataType::UUID,
            DataType::Text,
            DataType::Json,
        ]
    }

    pub fn as_str(&self) -> &str {
        match self {
            DataType::String => "String",
            DataType::Integer => "Integer",
            DataType::Float => "Float",
            DataType::Boolean => "Boolean",
            DataType::DateTime => "DateTime",
            DataType::UUID => "UUID",
            DataType::Text => "Text",
            DataType::Json => "JSON",
        }
    }

    pub fn to_python_type(&self) -> &str {
        match self {
            DataType::String => "str",
            DataType::Integer => "int",
            DataType::Float => "float",
            DataType::Boolean => "bool",
            DataType::DateTime => "datetime",
            DataType::UUID => "UUID",
            DataType::Text => "str",
            DataType::Json => "dict",
        }
    }

    pub fn to_sqlalchemy_type(&self) -> &str {
        match self {
            DataType::String => "String(255)",
            DataType::Integer => "Integer",
            DataType::Float => "Float",
            DataType::Boolean => "Boolean",
            DataType::DateTime => "DateTime",
            DataType::UUID => "String(36)",
            DataType::Text => "Text",
            DataType::Json => "JSON",
        }
    }
}

// ─── Model Field ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelField {
    pub id: String,
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub unique: bool,
    pub primary_key: bool,
    pub default_value: Option<String>,
}

impl ModelField {
    #[allow(dead_code)]
    pub fn new(name: String, data_type: DataType) -> Self {
        ModelField {
            id: Uuid::new_v4().to_string(),
            name,
            data_type,
            nullable: true,
            unique: false,
            primary_key: false,
            default_value: None,
        }
    }
}

// ─── Model (DB Table / Pydantic class) ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub fields: Vec<ModelField>,
}

impl Model {
    pub fn new(name: String) -> Self {
        Model {
            id: Uuid::new_v4().to_string(),
            name,
            fields: Vec::new(),
        }
    }
}

// ─── HTTP Methods ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    WebSocket,
}

impl HttpMethod {
    pub fn variants() -> Vec<HttpMethod> {
        vec![
            HttpMethod::GET,
            HttpMethod::POST,
            HttpMethod::PUT,
            HttpMethod::PATCH,
            HttpMethod::DELETE,
            HttpMethod::WebSocket,
        ]
    }

    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::WebSocket => "WS",
        }
    }

    #[allow(dead_code)]
    pub fn color_hint(&self) -> &str {
        match self {
            HttpMethod::GET => "green",
            HttpMethod::POST => "blue",
            HttpMethod::PUT => "yellow",
            HttpMethod::PATCH => "magenta",
            HttpMethod::DELETE => "red",
            HttpMethod::WebSocket => "cyan",
        }
    }
}

// ─── CRUD Operation ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CrudOp {
    Create,
    ReadOne,
    ReadAll,
    Update,
    Delete,
    Custom,
}

impl CrudOp {
    pub fn as_str(&self) -> &str {
        match self {
            CrudOp::Create => "Create",
            CrudOp::ReadOne => "Read One",
            CrudOp::ReadAll => "Read All",
            CrudOp::Update => "Update",
            CrudOp::Delete => "Delete",
            CrudOp::Custom => "Custom",
        }
    }
}

// ─── Endpoint ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: String,
    pub path: String,
    pub method: HttpMethod,
    pub crud_op: CrudOp,
    pub linked_model: Option<String>, // model id
    pub description: String,
    pub requires_auth: bool,
    pub path_params: Vec<String>,
    pub query_params: Vec<String>,
    pub body_params: Vec<(String, String)>, // (key, type)
    pub tags: Vec<String>,
}

impl Endpoint {
    pub fn new(path: String, method: HttpMethod) -> Self {
        Endpoint {
            id: Uuid::new_v4().to_string(),
            path,
            method,
            crud_op: CrudOp::Custom,
            linked_model: None,
            description: String::new(),
            requires_auth: false,
            path_params: Vec::new(),
            query_params: Vec::new(),
            body_params: Vec::new(),
            tags: Vec::new(),
        }
    }
}

// ─── Auth Config ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum AuthStrategy {
    JWT,
    Session,
    APIKey,
}

impl AuthStrategy {
    pub fn as_str(&self) -> &str {
        match self {
            AuthStrategy::JWT => "JWT Bearer Token",
            AuthStrategy::Session => "Session Cookie",
            AuthStrategy::APIKey => "API Key Header",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub strategy: AuthStrategy,
    pub token_expiry_minutes: u32,
    pub refresh_token: bool,
    pub user_model_name: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig {
            enabled: false,
            strategy: AuthStrategy::JWT,
            token_expiry_minutes: 30,
            refresh_token: true,
            user_model_name: "User".to_string(),
        }
    }
}

// ─── Fake DB Row ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeDbRow {
    pub id: u64,
    pub values: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeDbTable {
    pub model_id: String,
    pub rows: Vec<FakeDbRow>,
    pub next_id: u64,
}

impl FakeDbTable {
    pub fn new(model_id: String) -> Self {
        FakeDbTable {
            model_id,
            rows: Vec::new(),
            next_id: 1,
        }
    }
}

// ─── Project ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub description: String,
    pub models: Vec<Model>,
    pub endpoints: Vec<Endpoint>,
    pub auth_config: AuthConfig,
    pub fake_db: Vec<FakeDbTable>,
}

impl Project {
    pub fn new(name: String) -> Self {
        Project {
            name,
            description: String::new(),
            models: Vec::new(),
            endpoints: Vec::new(),
            auth_config: AuthConfig::default(),
            fake_db: Vec::new(),
        }
    }

    pub fn get_model_by_id(&self, id: &str) -> Option<&Model> {
        self.models.iter().find(|m| m.id == id)
    }

    #[allow(dead_code)]
    pub fn get_model_by_name(&self, name: &str) -> Option<&Model> {
        self.models.iter().find(|m| m.name == name)
    }

    pub fn get_fake_table(&self, model_id: &str) -> Option<&FakeDbTable> {
        self.fake_db.iter().find(|t| t.model_id == model_id)
    }

    pub fn get_fake_table_mut(&mut self, model_id: &str) -> Option<&mut FakeDbTable> {
        self.fake_db.iter_mut().find(|t| t.model_id == model_id)
    }
}

use std::fs;
use std::path::Path;
use anyhow::Result;
use crate::core::models::*;

pub fn export_project(project: &Project, output_dir: &str) -> Result<Vec<String>> {
    let root = Path::new(output_dir);
    let parent = root.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let staging = parent.join(format!(".backforge-export-staging-{}", uuid::Uuid::new_v4()));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }

    let generated_files = build_export_tree(project, &staging, output_dir)?;

    if root.exists() {
        let backup = parent.join(format!(".backforge-export-backup-{}", uuid::Uuid::new_v4()));
        if backup.exists() {
            fs::remove_dir_all(&backup)?;
        }

        fs::rename(root, &backup)?;
        match fs::rename(&staging, root) {
            Ok(_) => {
                let _ = fs::remove_dir_all(&backup);
            }
            Err(e) => {
                let _ = fs::rename(&backup, root);
                let _ = fs::remove_dir_all(&staging);
                return Err(e.into());
            }
        }
    } else {
        fs::rename(&staging, root)?;
    }

    Ok(generated_files)
}

fn build_export_tree(project: &Project, root: &Path, output_dir: &str) -> Result<Vec<String>> {
    let mut generated_files: Vec<String> = Vec::new();

    // Create directory structure
    let dirs = [
        "",
        "app",
        "app/models",
        "app/schemas",
        "app/routers",
        "app/auth",
        "app/db",
        "app/core",
        "tests",
    ];
    for dir in &dirs {
        fs::create_dir_all(root.join(dir))?;
    }

    // ── main.py ──────────────────────────────────────────────────────────────
    let main_py = generate_main_py(project);
    write_file(root, "main.py", &main_py, &mut generated_files)?;

    // ── requirements.txt ──────────────────────────────────────────────────────
    let reqs = generate_requirements(project);
    write_file(root, "requirements.txt", &reqs, &mut generated_files)?;

    // ── .env.example ──────────────────────────────────────────────────────────
    write_file(root, ".env.example", &generate_env_example(project), &mut generated_files)?;

    // ── Dockerfile ────────────────────────────────────────────────────────────
    write_file(root, "Dockerfile", &generate_dockerfile(), &mut generated_files)?;

    // ── docker-compose.yml ────────────────────────────────────────────────────
    write_file(root, "docker-compose.yml", &generate_docker_compose(&project.name), &mut generated_files)?;

    // ── app/__init__.py ────────────────────────────────────────────────────────
    write_file(root, "app/__init__.py", "", &mut generated_files)?;

    // ── app/db/database.py ────────────────────────────────────────────────────
    write_file(root, "app/db/__init__.py", "", &mut generated_files)?;
    write_file(root, "app/db/database.py", &generate_database_py(), &mut generated_files)?;

    // ── app/core/config.py ────────────────────────────────────────────────────
    write_file(root, "app/core/__init__.py", "", &mut generated_files)?;
    write_file(root, "app/core/config.py", &generate_config_py(), &mut generated_files)?;

    // ── app/models/__init__.py + per-model files ───────────────────────────────
    write_file(root, "app/models/__init__.py", &generate_models_init(project), &mut generated_files)?;
    for model in &project.models {
        let mn = safe_model_name(&model.name);
        let fname = format!("app/models/{}.py", mn);
        write_file(root, &fname, &generate_model_file(model), &mut generated_files)?;
    }

    // ── app/schemas/__init__.py + per-model schemas ───────────────────────────
    write_file(root, "app/schemas/__init__.py", &generate_schemas_init(project), &mut generated_files)?;
    for model in &project.models {
        let mn = safe_model_name(&model.name);
        let fname = format!("app/schemas/{}.py", mn);
        write_file(root, &fname, &generate_schema_file(model), &mut generated_files)?;
    }

    // ── Auth ──────────────────────────────────────────────────────────────────
    write_file(root, "app/auth/__init__.py", "", &mut generated_files)?;
    if project.auth_config.enabled {
        write_file(root, "app/auth/auth.py", &generate_auth_py(project), &mut generated_files)?;
        write_file(root, "app/auth/jwt.py", &generate_jwt_py(project), &mut generated_files)?;
        write_file(root, "app/routers/auth.py", &generate_auth_router(), &mut generated_files)?;
        write_file(root, "app/models/user.py", &generate_user_model(), &mut generated_files)?;
    }

    // ── app/routers/ ──────────────────────────────────────────────────────────
    write_file(root, "app/routers/__init__.py", "", &mut generated_files)?;
    for model in &project.models {
        let model_eps: Vec<&Endpoint> = project
            .endpoints
            .iter()
            .filter(|e| e.linked_model.as_ref().map(|id| id == &model.id).unwrap_or(false))
            .collect();
        let mn = safe_model_name(&model.name);
        let fname = format!("app/routers/{}.py", mn);
        write_file(root, &fname, &generate_router_file(model, &model_eps, project), &mut generated_files)?;
    }

    // Custom / unlinked endpoints
    let custom_eps: Vec<&Endpoint> = project
        .endpoints
        .iter()
        .filter(|e| e.linked_model.is_none() && !e.tags.contains(&"auth".to_string()))
        .collect();
    if !custom_eps.is_empty() {
        write_file(root, "app/routers/custom.py", &generate_custom_router(&custom_eps), &mut generated_files)?;
    }

    // ── tests/ ────────────────────────────────────────────────────────────────
    write_file(root, "tests/__init__.py", "", &mut generated_files)?;
    write_file(root, "tests/conftest.py", &generate_conftest(project), &mut generated_files)?;
    for model in &project.models {
        let fname = format!("tests/test_{}.py", safe_model_name(&model.name));
        write_file(root, &fname, &generate_test_file(model, project), &mut generated_files)?;
    }

    // ── README.md ─────────────────────────────────────────────────────────────
    write_file(root, "README.md", &generate_readme(project, output_dir), &mut generated_files)?;

    Ok(generated_files)
}

fn write_file(root: &Path, rel: &str, content: &str, log: &mut Vec<String>) -> Result<()> {
    let full = root.join(rel);
    fs::write(&full, content)?;
    log.push(rel.to_string());
    Ok(())
}

/// Returns a valid Python module name (lowercase, underscores only)
fn safe_model_name(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "_")
        .replace('-', "_")
        .replace('.', "_")
}

// ── main.py ──────────────────────────────────────────────────────────────────

fn generate_main_py(project: &Project) -> String {
    let mut router_imports = String::new();
    let mut router_includes = String::new();

    // One router per model (only include models that actually have endpoints linked)
    for model in &project.models {
        let has_eps = project.endpoints.iter().any(|e| {
            e.linked_model.as_ref().map(|id| id == &model.id).unwrap_or(false)
        });
        let mn = safe_model_name(&model.name);
        router_imports.push_str(&format!("from app.routers import {}\n", mn));
        router_includes.push_str(&format!(
            "app.include_router({mn}.router, prefix=\"/{mn}\", tags=[\"{name}\"])\n",
            mn = mn,
            name = model.name,
        ));
        let _ = has_eps;
    }

    // Custom / unlinked endpoints
    let has_custom = project.endpoints.iter().any(|e| {
        e.linked_model.is_none() && !e.tags.contains(&"auth".to_string())
    });
    if has_custom {
        router_imports.push_str("from app.routers import custom\n");
        router_includes.push_str("app.include_router(custom.router, tags=[\"custom\"])\n");
    }

    // Auth
    if project.auth_config.enabled {
        router_imports.push_str("from app.routers import auth as auth_router\n");
        router_includes.push_str("app.include_router(auth_router.router, prefix=\"/auth\", tags=[\"auth\"])\n");
    }

    // Build the router section comment
    let mut router_comments = String::new();
    for model in &project.models {
        let mn = safe_model_name(&model.name);
        let ep_count = project.endpoints.iter()
            .filter(|e| e.linked_model.as_ref().map(|id| id == &model.id).unwrap_or(false))
            .count();
        router_comments.push_str(&format!(
            "# /{mn}  →  app/routers/{mn}.py  ({ep_count} endpoints)\n",
            mn = mn, ep_count = ep_count
        ));
    }
    if has_custom {
        let n = project.endpoints.iter()
            .filter(|e| e.linked_model.is_none() && !e.tags.contains(&"auth".to_string()))
            .count();
        router_comments.push_str(&format!("# /custom  →  app/routers/custom.py  ({} endpoints)\n", n));
    }
    if project.auth_config.enabled {
        router_comments.push_str("# /auth    →  app/routers/auth.py  (register/login/me/refresh/logout)\n");
    }

    format!(
        r#"# ─────────────────────────────────────────────────────────────────────────────
# {name} — generated by BackForge
#
# HOW THIS PROJECT IS WIRED:
#   main.py              ← you are here. Creates the FastAPI app and attaches routers.
#   app/routers/         ← one file per model = your actual endpoints
#   app/models/          ← SQLAlchemy ORM classes (maps to DB tables)
#   app/schemas/         ← Pydantic request/response shapes (validation + docs)
#   app/db/database.py   ← DB engine + get_db() session dependency
#   app/core/config.py   ← reads .env settings (DATABASE_URL, SECRET_KEY, etc.)
#   app/auth/            ← JWT auth helpers (only present if auth enabled)
#   tests/               ← pytest test stubs, one file per model
#
# ROUTES REGISTERED:
{router_comments}#
# Run:  uvicorn main:app --reload
# Docs: http://localhost:8000/docs
# ─────────────────────────────────────────────────────────────────────────────
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from app.db.database import engine, Base

# Import all models so Base.metadata knows about them before create_all
import app.models  # noqa: F401
{router_imports}

# Create tables — must happen after model imports so SQLAlchemy sees all tables
Base.metadata.create_all(bind=engine)

app = FastAPI(
    title="{name}",
    description="Generated by BackForge — edit freely",
    version="0.1.0",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# ── Routers ───────────────────────────────────────────────────────────────────
{router_includes}
"#,
        name = project.name,
        router_imports = router_imports.trim_end(),
        router_includes = router_includes.trim_end(),
        router_comments = router_comments,
    )
}

// ── requirements.txt ─────────────────────────────────────────────────────────

fn generate_requirements(project: &Project) -> String {
    let mut reqs = vec![
        "fastapi>=0.110.0",
        "uvicorn[standard]>=0.27.0",
        "sqlalchemy>=2.0.0",
        "pydantic>=2.0.0",
        "pydantic-settings>=2.0.0",
        "alembic>=1.13.0",
        "python-dotenv>=1.0.0",
        "httpx>=0.27.0",  // for testing
        "pytest>=8.0.0",
        "pytest-asyncio>=0.23.0",
    ];

    if project.auth_config.enabled {
        reqs.push("python-jose[cryptography]>=3.3.0");
        reqs.push("passlib[bcrypt]>=1.7.4");
        reqs.push("python-multipart>=0.0.9");
    }

    reqs.join("\n")
}

// ── .env.example ─────────────────────────────────────────────────────────────

fn generate_env_example(project: &Project) -> String {
    let mut env = format!(
        r#"# ─── Database ────────────────────────────────────────────
# Uncomment the one you want to use:

# SQLite (default, good for dev)
DATABASE_URL=sqlite:///./app.db

# PostgreSQL
# DATABASE_URL=postgresql://user:password@localhost:5432/{db}

# MySQL
# DATABASE_URL=mysql+pymysql://user:password@localhost:3306/{db}

# ─── App ─────────────────────────────────────────────────
APP_NAME={name}
DEBUG=true
"#,
        db = project.name.to_lowercase().replace(' ', "_"),
        name = project.name,
    );

    if project.auth_config.enabled {
        env.push_str(
            r#"
# ─── Auth (JWT) ──────────────────────────────────────────
SECRET_KEY=your-super-secret-key-change-this-in-production
ALGORITHM=HS256
ACCESS_TOKEN_EXPIRE_MINUTES=30
"#,
        );
    }

    env
}

// ── Dockerfile ───────────────────────────────────────────────────────────────

fn generate_dockerfile() -> String {
    r#"FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 8000

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
"#
    .to_string()
}

fn generate_docker_compose(name: &str) -> String {
    let svc = name.to_lowercase().replace(' ', "_");
    format!(
        r#"version: "3.8"

services:
  api:
    build: .
    ports:
      - "8000:8000"
    environment:
      - DATABASE_URL=postgresql://postgres:postgres@db:5432/{svc}
    depends_on:
      - db
    volumes:
      - .:/app

  db:
    image: postgres:16
    environment:
      POSTGRES_DB: {svc}
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
"#,
        svc = svc
    )
}

// ── database.py ──────────────────────────────────────────────────────────────

fn generate_database_py() -> String {
    r#"from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker, declarative_base
from app.core.config import settings

engine = create_engine(
    settings.DATABASE_URL,
    connect_args={"check_same_thread": False} if "sqlite" in settings.DATABASE_URL else {},
)

SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

Base = declarative_base()


def get_db():
    """Dependency: yields a DB session and closes it after request."""
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()
"#
    .to_string()
}

// ── config.py ────────────────────────────────────────────────────────────────

fn generate_config_py() -> String {
    r#"from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    DATABASE_URL: str = "sqlite:///./app.db"
    APP_NAME: str = "BackForge App"
    DEBUG: bool = True
    SECRET_KEY: str = "change-me-in-production"
    ALGORITHM: str = "HS256"
    ACCESS_TOKEN_EXPIRE_MINUTES: int = 30

    class Config:
        env_file = ".env"


settings = Settings()
"#
    .to_string()
}

// ── models/__init__.py ────────────────────────────────────────────────────────

fn generate_models_init(project: &Project) -> String {
    let mut s = String::from("from app.db.database import Base  # noqa: F401\n");
    if project.auth_config.enabled {
        s.push_str("from app.models.user import User  # noqa: F401\n");
    }
    for model in &project.models {
        s.push_str(&format!(
            "from app.models.{} import {}  # noqa: F401\n",
            safe_model_name(&model.name),
            model.name
        ));
    }
    s
}

// ── models/<model>.py ─────────────────────────────────────────────────────────

fn generate_model_file(model: &Model) -> String {
    // Collect which SQLAlchemy column *types* are needed
    let mut col_types: Vec<&str> = vec!["Integer"]; // always need Integer for auto-id fallback
    for f in &model.fields {
        let t = match f.data_type {
            DataType::String | DataType::UUID => "String",
            DataType::Integer => "Integer",
            DataType::Float   => "Float",
            DataType::Boolean => "Boolean",
            DataType::DateTime => "DateTime",
            DataType::Text    => "Text",
            DataType::Json    => "JSON",
        };
        if !col_types.contains(&t) { col_types.push(t); }
    }
    col_types.sort();
    col_types.dedup();

    // Column is always needed (used for every field declaration)
    // Build:  from sqlalchemy import Column, Boolean, Integer, String, ...
    let sa_imports = format!("Column, {}", col_types.join(", "));

    let mut out = format!("from sqlalchemy import {}\n", sa_imports);
    out.push_str("from app.db.database import Base\n");
    if model.fields.iter().any(|f| f.data_type == DataType::DateTime) {
        out.push_str("from datetime import datetime\n");
    }

    out.push_str(&format!("\n\nclass {}(Base):\n", model.name));
    out.push_str(&format!("    __tablename__ = \"{}\"\n\n", model.name.to_lowercase()));

    // Always ensure there is a primary key column
    let has_pk = model.fields.iter().any(|f| f.primary_key);
    if model.fields.is_empty() || !has_pk {
        out.push_str("    id = Column(Integer, primary_key=True, index=True, autoincrement=True)\n");
    }

    for field in &model.fields {
        let col_type = match field.data_type {
            DataType::String   => "String(255)".to_string(),
            DataType::Integer  => "Integer".to_string(),
            DataType::Float    => "Float".to_string(),
            DataType::Boolean  => "Boolean".to_string(),
            DataType::DateTime => "DateTime".to_string(),
            DataType::UUID     => "String(36)".to_string(),
            DataType::Text     => "Text".to_string(),
            DataType::Json     => "JSON".to_string(),
        };

        let mut args = col_type;
        if field.primary_key {
            args.push_str(", primary_key=True, index=True");
            if field.data_type == DataType::Integer {
                args.push_str(", autoincrement=True");
            }
        } else {
            if !field.nullable { args.push_str(", nullable=False"); }
            if field.unique    { args.push_str(", unique=True"); }
        }

        // Use lowercase field name always — Python convention
        out.push_str(&format!("    {} = Column({})\n", field.name.to_lowercase(), args));
    }

    out
}

// ── schemas/<model>.py ────────────────────────────────────────────────────────

fn generate_schemas_init(project: &Project) -> String {
    let mut s = String::new();
    for model in &project.models {
        s.push_str(&format!(
            "from app.schemas.{} import {}Base, {}Create, {}Update, {}Response  # noqa: F401\n",
            safe_model_name(&model.name),
            model.name, model.name, model.name, model.name,
        ));
    }
    s
}

fn generate_schema_file(model: &Model) -> String {
    let mut imports = vec![
        "from pydantic import BaseModel".to_string(),
        "from typing import Optional".to_string(),
    ];

    if model.fields.iter().any(|f| f.data_type == DataType::DateTime) {
        imports.push("from datetime import datetime".to_string());
    }

    let mut lines = imports.join("\n");
    lines.push_str("\n\n");

    // Base schema — all field names lowercased to match SQLAlchemy columns
    lines.push_str(&format!("class {}Base(BaseModel):\n", model.name));
    let non_pk_fields: Vec<&ModelField> = model.fields.iter().filter(|f| !f.primary_key).collect();
    if non_pk_fields.is_empty() {
        lines.push_str("    pass\n");
    } else {
        for f in &non_pk_fields {
            let fname = f.name.to_lowercase();
            let py_type = f.data_type.to_python_type();
            if f.nullable {
                lines.push_str(&format!("    {}: Optional[{}] = None\n", fname, py_type));
            } else {
                lines.push_str(&format!("    {}: {}\n", fname, py_type));
            }
        }
    }

    // Create schema
    lines.push_str(&format!("\n\nclass {}Create({}Base):\n", model.name, model.name));
    if non_pk_fields.is_empty() {
        // Edge case: model has only primary key.
        // If PK is non-integer, clients usually must provide it explicitly.
        if let Some(pk) = model.fields.iter().find(|f| f.primary_key) {
            if !matches!(pk.data_type, DataType::Integer) {
                lines.push_str(&format!(
                    "    {}: {}\n",
                    pk.name.to_lowercase(),
                    pk.data_type.to_python_type()
                ));
            } else {
                lines.push_str("    pass\n");
            }
        } else {
            lines.push_str("    pass\n");
        }
    } else {
        lines.push_str("    pass\n");
    }

    // Update schema — all optional
    lines.push_str(&format!("\n\nclass {}Update(BaseModel):\n", model.name));
    if non_pk_fields.is_empty() {
        lines.push_str("    pass\n");
    } else {
        for f in &non_pk_fields {
            let fname = f.name.to_lowercase();
            let py_type = f.data_type.to_python_type();
            lines.push_str(&format!("    {}: Optional[{}] = None\n", fname, py_type));
        }
    }

    // Response schema
    lines.push_str(&format!("\n\nclass {}Response({}Base):\n", model.name, model.name));
    if let Some(pk) = model.fields.iter().find(|f| f.primary_key) {
        lines.push_str(&format!("    {}: {}\n\n", pk.name.to_lowercase(), pk.data_type.to_python_type()));
    } else {
        lines.push_str("    id: int\n\n");
    }
    lines.push_str("    class Config:\n        from_attributes = True\n");

    lines
}

// ── models/user.py (auth) ─────────────────────────────────────────────────────

fn generate_user_model() -> String {
    r#"from sqlalchemy import Column, Integer, String, Boolean, DateTime
from app.db.database import Base
from datetime import datetime


class User(Base):
    __tablename__ = "users"

    id = Column(Integer, primary_key=True, index=True, autoincrement=True)
    username = Column(String(100), unique=True, nullable=False, index=True)
    email = Column(String(255), unique=True, nullable=False)
    hashed_password = Column(String(255), nullable=False)
    is_active = Column(Boolean, default=True)
    is_admin = Column(Boolean, default=False)
    created_at = Column(DateTime, default=datetime.utcnow)
"#
    .to_string()
}

// ── auth/auth.py ──────────────────────────────────────────────────────────────

fn generate_auth_py(project: &Project) -> String {
    let _ = project;
    r#"from passlib.context import CryptContext
from sqlalchemy.orm import Session
from app.models.user import User
from app.auth.jwt import create_access_token, verify_token
from fastapi import Depends, HTTPException, status
from fastapi.security import OAuth2PasswordBearer

pwd_context = CryptContext(schemes=["bcrypt"], deprecated="auto")
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="/auth/login")


def hash_password(password: str) -> str:
    return pwd_context.hash(password)


def verify_password(plain: str, hashed: str) -> bool:
    return pwd_context.verify(plain, hashed)


def authenticate_user(db: Session, username: str, password: str):
    user = db.query(User).filter(User.username == username).first()
    if not user or not verify_password(password, user.hashed_password):
        return None
    return user


def get_current_user(token: str = Depends(oauth2_scheme), db: Session = Depends(lambda: None)):
    credentials_exception = HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )
    payload = verify_token(token)
    if payload is None:
        raise credentials_exception
    username: str = payload.get("sub")
    if username is None:
        raise credentials_exception
    user = db.query(User).filter(User.username == username).first()
    if user is None:
        raise credentials_exception
    return user
"#
    .to_string()
}

fn generate_jwt_py(project: &Project) -> String {
    let _ = project;
    r#"from datetime import datetime, timedelta
from jose import JWTError, jwt
from app.core.config import settings


def create_access_token(data: dict) -> str:
    to_encode = data.copy()
    expire = datetime.utcnow() + timedelta(minutes=settings.ACCESS_TOKEN_EXPIRE_MINUTES)
    to_encode.update({"exp": expire})
    return jwt.encode(to_encode, settings.SECRET_KEY, algorithm=settings.ALGORITHM)


def create_refresh_token(data: dict) -> str:
    to_encode = data.copy()
    expire = datetime.utcnow() + timedelta(days=7)
    to_encode.update({"exp": expire, "type": "refresh"})
    return jwt.encode(to_encode, settings.SECRET_KEY, algorithm=settings.ALGORITHM)


def verify_token(token: str) -> dict | None:
    try:
        payload = jwt.decode(token, settings.SECRET_KEY, algorithms=[settings.ALGORITHM])
        return payload
    except JWTError:
        return None
"#
    .to_string()
}

fn generate_auth_router() -> String {
    r#"from fastapi import APIRouter, Depends, HTTPException, status
from fastapi.security import OAuth2PasswordRequestForm
from sqlalchemy.orm import Session
from app.db.database import get_db
from app.auth.auth import authenticate_user, hash_password, get_current_user
from app.auth.jwt import create_access_token, create_refresh_token
from app.models.user import User
from pydantic import BaseModel

router = APIRouter()


class UserCreate(BaseModel):
    username: str
    email: str
    password: str


class Token(BaseModel):
    access_token: str
    refresh_token: str
    token_type: str


@router.post("/register", response_model=dict, status_code=201)
def register(user_data: UserCreate, db: Session = Depends(get_db)):
    existing = db.query(User).filter(User.username == user_data.username).first()
    if existing:
        raise HTTPException(status_code=400, detail="Username already taken")
    user = User(
        username=user_data.username,
        email=user_data.email,
        hashed_password=hash_password(user_data.password),
    )
    db.add(user)
    db.commit()
    db.refresh(user)
    return {"id": user.id, "username": user.username, "email": user.email}


@router.post("/login", response_model=Token)
def login(form_data: OAuth2PasswordRequestForm = Depends(), db: Session = Depends(get_db)):
    user = authenticate_user(db, form_data.username, form_data.password)
    if not user:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Incorrect username or password",
        )
    access_token = create_access_token({"sub": user.username})
    refresh_token = create_refresh_token({"sub": user.username})
    return {"access_token": access_token, "refresh_token": refresh_token, "token_type": "bearer"}


@router.post("/logout")
def logout():
    # In stateless JWT, logout is handled client-side.
    # To implement server-side revocation, use a token blocklist (Redis recommended).
    return {"message": "Logged out successfully"}


@router.get("/me")
def me(current_user: User = Depends(get_current_user)):
    return {"id": current_user.id, "username": current_user.username, "email": current_user.email}


@router.post("/refresh")
def refresh(token: str, db: Session = Depends(get_db)):
    from app.auth.jwt import verify_token
    payload = verify_token(token)
    if not payload or payload.get("type") != "refresh":
        raise HTTPException(status_code=401, detail="Invalid refresh token")
    access_token = create_access_token({"sub": payload["sub"]})
    return {"access_token": access_token, "token_type": "bearer"}
"#
    .to_string()
}

// ── routers/<model>.py ────────────────────────────────────────────────────────

fn generate_router_file(model: &Model, endpoints: &[&Endpoint], project: &Project) -> String {
    // mn must be a valid Python identifier: lowercase, underscores only
    let mn = model.name.to_lowercase()
        .replace(' ', "_").replace('-', "_").replace('.', "_");
    let mn_cap = &model.name;

    let auth_import = if project.auth_config.enabled {
        "from app.auth.auth import get_current_user\nfrom app.models.user import User\n"
    } else {
        ""
    };

    let mut lines = format!(
        r#"from fastapi import APIRouter, Depends, HTTPException, status
from fastapi import Query
from sqlalchemy.orm import Session
from typing import List, Optional
{auth_import}from app.db.database import get_db
from app.models.{mn} import {mn_cap}
from app.schemas.{mn} import {mn_cap}Create, {mn_cap}Update, {mn_cap}Response

router = APIRouter()

"#,
        auth_import = auth_import,
        mn = mn,
        mn_cap = mn_cap,
    );

    // Only generate default CRUD if there are genuinely no endpoints linked
    // to this model AND no custom endpoints exist that reference this model.
    // This way user-defined endpoints are always respected.
    if endpoints.is_empty() {
        // Check if user created ANY endpoint for this model (even unlinked ones
        // can't count — only truly linked ones qualify for auto-CRUD suppression).
        // Since endpoints is already the filtered list, empty means user made none.
        // We do NOT auto-generate default CRUD — leave it empty with a comment.
        lines.push_str(&format!(
            "# No endpoints defined for {}.\n",
            mn_cap
        ));
        lines.push_str("# Link endpoints to this model in BackForge, or add routes here manually.\n");
    } else {
        for ep in endpoints {
            lines.push_str(&generate_endpoint_handler(ep, model, project));
            lines.push('\n');
        }
    }

    lines
}

#[allow(dead_code)]
fn generate_default_crud(model: &Model) -> String {
    let mn = model.name.to_lowercase()
        .replace(' ', "_").replace('-', "_").replace('.', "_");
    let mn_cap = &model.name;

    format!(
        r#"@router.get("/", response_model=List[{mn_cap}Response])
def get_all_{mn}s(skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    return db.query({mn_cap}).offset(skip).limit(limit).all()


@router.get("/{{{mn}_id}}", response_model={mn_cap}Response)
def get_{mn}({mn}_id: int, db: Session = Depends(get_db)):
    item = db.query({mn_cap}).filter({mn_cap}.id == {mn}_id).first()
    if not item:
        raise HTTPException(status_code=404, detail="{mn_cap} not found")
    return item


@router.post("/", response_model={mn_cap}Response, status_code=201)
def create_{mn}(data: {mn_cap}Create, db: Session = Depends(get_db)):
    db_item = {mn_cap}(**data.model_dump())
    db.add(db_item)
    db.commit()
    db.refresh(db_item)
    return db_item


@router.put("/{{{mn}_id}}", response_model={mn_cap}Response)
def update_{mn}({mn}_id: int, data: {mn_cap}Update, db: Session = Depends(get_db)):
    item = db.query({mn_cap}).filter({mn_cap}.id == {mn}_id).first()
    if not item:
        raise HTTPException(status_code=404, detail="{mn_cap} not found")
    for k, v in data.model_dump(exclude_unset=True).items():
        setattr(item, k, v)
    db.commit()
    db.refresh(item)
    return item


@router.delete("/{{{mn}_id}}", status_code=204)
def delete_{mn}({mn}_id: int, db: Session = Depends(get_db)):
    item = db.query({mn_cap}).filter({mn_cap}.id == {mn}_id).first()
    if not item:
        raise HTTPException(status_code=404, detail="{mn_cap} not found")
    db.delete(item)
    db.commit()
"#,
        mn = mn,
        mn_cap = mn_cap,
    )
}

fn generate_endpoint_handler(ep: &Endpoint, model: &Model, project: &Project) -> String {
    let method = ep.method.as_str().to_lowercase();
    let mn = model.name.to_lowercase()
        .replace(' ', "_").replace('-', "_").replace('.', "_");
    let mn_cap = &model.name;

    // Build a valid Python function name from the path
    let path_part = ep.path
        .trim_matches('/')
        .replace('/', "_")
        .replace('{', "")
        .replace('}', "")
        .replace('-', "_")
        .replace('.', "_");
    // If path was "/" or becomes empty, use model name
    let path_part = if path_part.is_empty() {
        mn.clone()
    } else {
        path_part
    };
    let func_name = format!("{}_{}", method, path_part);

    let return_type = match ep.crud_op {
        CrudOp::ReadAll => format!("List[{}Response]", mn_cap),
        CrudOp::Delete => "dict".to_string(),
        _ => format!("{}Response", mn_cap),
    };

    let body = match ep.crud_op {
        CrudOp::Create => format!(
            "    db_item = {mn_cap}(**data.model_dump())\n    db.add(db_item)\n    db.commit()\n    db.refresh(db_item)\n    return db_item"
        ),
        CrudOp::ReadAll => {
            let mut s = format!("    query = db.query({mn_cap})\n");
            for qp in &ep.query_params {
                s.push_str(&format!("    if {q} is not None:\n        query = query.filter({mn_cap}.{q} == {q})\n", q = qp, mn_cap = mn_cap));
            }
            s.push_str("    return query.offset(0).limit(100).all()");
            s
        }
        CrudOp::ReadOne => {
            if let Some(pk_param) = ep.path_params.first().cloned() {
                format!(
                    "    item = db.query({mn_cap}).filter({mn_cap}.id == {pk}).first()\n    if not item:\n        raise HTTPException(status_code=404, detail=\"{mn_cap} not found\")\n    return item",
                    mn_cap = mn_cap, pk = pk_param
                )
            } else if let Some(first_q) = ep.query_params.first() {
                format!(
                    "    if {q} is None:\n        raise HTTPException(status_code=400, detail=\"Missing query parameter: {q}\")\n    item = db.query({mn_cap}).filter({mn_cap}.{q} == {q}).first()\n    if not item:\n        raise HTTPException(status_code=404, detail=\"{mn_cap} not found\")\n    return item",
                    q = first_q,
                    mn_cap = mn_cap
                )
            } else {
                format!(
                    "    item = db.query({mn_cap}).first()\n    if not item:\n        raise HTTPException(status_code=404, detail=\"{mn_cap} not found\")\n    return item",
                    mn_cap = mn_cap
                )
            }
        }
        CrudOp::Update => {
            let pk_param = ep.path_params.first().cloned().unwrap_or_else(|| format!("{}_id", mn));
            format!(
                "    item = db.query({mn_cap}).filter({mn_cap}.id == {pk}).first()\n    if not item:\n        raise HTTPException(status_code=404, detail=\"{mn_cap} not found\")\n    for k, v in data.model_dump(exclude_unset=True).items():\n        setattr(item, k, v)\n    db.commit()\n    db.refresh(item)\n    return item",
                mn_cap = mn_cap, pk = pk_param
            )
        }
        CrudOp::Delete => {
            if let Some(pk_param) = ep.path_params.first().cloned() {
                format!(
                    "    item = db.query({mn_cap}).filter({mn_cap}.id == {pk}).first()\n    if not item:\n        raise HTTPException(status_code=404, detail=\"{mn_cap} not found\")\n    db.delete(item)\n    db.commit()\n    return {{\"ok\": True}}",
                    mn_cap = mn_cap, pk = pk_param
                )
            } else if !ep.query_params.is_empty() {
                let mut s = format!("    query = db.query({mn_cap})\n", mn_cap = mn_cap);
                for qp in &ep.query_params {
                    s.push_str(&format!("    if {q} is not None:\n        query = query.filter({mn_cap}.{q} == {q})\n", q = qp, mn_cap = mn_cap));
                }
                s.push_str("    deleted = query.delete(synchronize_session=False)\n    db.commit()\n    return {\"deleted\": deleted}");
                s
            } else {
                "    raise HTTPException(status_code=400, detail=\"Delete endpoint needs path or query params\")".to_string()
            }
        }
        CrudOp::Custom => "    # TODO: implement custom logic\n    pass".to_string(),
    };

    let desc = if !ep.description.is_empty() {
        format!("\n    \"\"\"{}\"\"\"\n", ep.description)
    } else {
        String::new()
    };

    // Build param list cleanly — no leading/trailing commas
    let mut params: Vec<String> = Vec::new();
    for p in &ep.path_params {
        params.push(format!("{}: int", p));
    }
    for q in &ep.query_params {
        let q_type = infer_query_type(model, q);
        params.push(format!("{}: Optional[{}] = Query(None)", q, q_type));
    }
    if matches!(ep.method, HttpMethod::POST | HttpMethod::PUT | HttpMethod::PATCH) {
        let schema_name = if matches!(ep.method, HttpMethod::PUT | HttpMethod::PATCH) {
            format!("{}Update", mn_cap)
        } else {
            format!("{}Create", mn_cap)
        };
        params.push(format!("data: {}", schema_name));
    }
    if ep.requires_auth && project.auth_config.enabled {
        params.push("current_user: User = Depends(get_current_user)".to_string());
    }
    params.push("db: Session = Depends(get_db)".to_string());

    let params_str = params.join(", ");

    format!(
        "@router.{}(\"{}\")\ndef {}({}) -> {}:\n{}{}\n",
        method,
        ep.path,
        func_name,
        params_str,
        return_type,
        desc,
        body,
    )
}

fn infer_query_type(model: &Model, param: &str) -> String {
    model
        .fields
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case(param))
    .map(|f| f.data_type.to_python_type().to_string())
    .unwrap_or_else(|| "str".to_string())
}

#[cfg(test)]
mod tests {
    use super::{export_project, generate_endpoint_handler, generate_schema_file};
    use crate::core::models::{CrudOp, DataType, Endpoint, HttpMethod, Model, ModelField, Project};

    fn sample_model() -> Model {
        Model {
            id: "m1".to_string(),
            name: "User".to_string(),
            fields: vec![
                ModelField {
                    id: "f1".to_string(),
                    name: "id".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                    unique: true,
                    primary_key: true,
                    default_value: None,
                },
                ModelField {
                    id: "f2".to_string(),
                    name: "email".to_string(),
                    data_type: DataType::String,
                    nullable: false,
                    unique: true,
                    primary_key: false,
                    default_value: None,
                },
            ],
        }
    }

    #[test]
    fn endpoint_handler_uses_query_params_for_get() {
        let model = sample_model();
        let ep = Endpoint {
            id: "e1".to_string(),
            path: "/".to_string(),
            method: HttpMethod::GET,
            crud_op: CrudOp::ReadAll,
            linked_model: Some(model.id.clone()),
            description: "".to_string(),
            requires_auth: false,
            path_params: vec![],
            query_params: vec!["email".to_string()],
            body_params: vec![],
            tags: vec![],
        };

        let out = generate_endpoint_handler(&ep, &model, &Project::new("P".to_string()));
        assert!(out.contains("email: Optional[str] = Query(None)"));
        assert!(out.contains("if email is not None"));
    }

    #[test]
    fn schema_handles_pk_only_non_integer_model() {
        let model = Model {
            id: "m2".to_string(),
            name: "ApiKey".to_string(),
            fields: vec![ModelField {
                id: "f1".to_string(),
                name: "key".to_string(),
                data_type: DataType::UUID,
                nullable: false,
                unique: true,
                primary_key: true,
                default_value: None,
            }],
        };

        let out = generate_schema_file(&model);
        assert!(out.contains("class ApiKeyCreate(ApiKeyBase):"));
        assert!(out.contains("key: UUID"));
    }

    #[test]
    fn export_replaces_existing_directory_atomically() {
        let project = Project::new("AtomicExport".to_string());
        let out = std::env::temp_dir().join(format!(
            "backforge_export_atomic_{}",
            uuid::Uuid::new_v4()
        ));

        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("stale.txt"), "old").unwrap();

        let generated = export_project(&project, out.to_str().unwrap()).unwrap();
        assert!(generated.iter().any(|f| f == "main.py"));
        assert!(out.join("main.py").exists());
        assert!(!out.join("stale.txt").exists());

        let _ = std::fs::remove_dir_all(&out);
    }
}

fn generate_custom_router(endpoints: &[&Endpoint]) -> String {
    let mut lines = "from fastapi import APIRouter\n\nrouter = APIRouter()\n\n".to_string();
    for (i, ep) in endpoints.iter().enumerate() {
        let method = ep.method.as_str().to_lowercase();
        let path_part = ep.path
            .trim_matches('/')
            .replace('/', "_")
            .replace('{', "").replace('}', "")
            .replace('-', "_").replace('.', "_");
        let path_part = if path_part.is_empty() {
            format!("route_{}", i)
        } else {
            path_part
        };
        let func_name = format!("{}_{}", method, path_part);
        lines.push_str(&format!(
            "@router.{}(\"{}\")\ndef {}():\n    # TODO: implement\n    pass\n\n",
            method, ep.path, func_name
        ));
    }
    lines
}

// ── tests/conftest.py ─────────────────────────────────────────────────────────

fn generate_conftest(project: &Project) -> String {
    let _ = project;
    r#"import pytest
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from app.db.database import Base, get_db
from main import app

SQLALCHEMY_DATABASE_URL = "sqlite:///./test.db"
engine = create_engine(SQLALCHEMY_DATABASE_URL, connect_args={"check_same_thread": False})
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


@pytest.fixture(scope="function", autouse=True)
def setup_db():
    Base.metadata.create_all(bind=engine)
    yield
    Base.metadata.drop_all(bind=engine)


@pytest.fixture()
def db():
    session = TestingSessionLocal()
    try:
        yield session
    finally:
        session.close()


@pytest.fixture()
def client(db):
    def override_get_db():
        try:
            yield db
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db
    with TestClient(app) as c:
        yield c
    app.dependency_overrides.clear()
"#
    .to_string()
}

fn generate_test_file(model: &Model, _project: &Project) -> String {
    let mn = safe_model_name(&model.name);

    format!(
        r#"import pytest
from fastapi.testclient import TestClient


def test_create_{mn}(client: TestClient):
    # TODO: fill in valid payload
    payload = {{}}
    response = client.post("/{mn}/", json=payload)
    assert response.status_code in (200, 201)


def test_get_all_{mn}s(client: TestClient):
    response = client.get("/{mn}/")
    assert response.status_code == 200
    assert isinstance(response.json(), list)


def test_get_{mn}_not_found(client: TestClient):
    response = client.get("/{mn}/99999")
    assert response.status_code == 404


def test_update_{mn}(client: TestClient):
    # Create first
    create_resp = client.post("/{mn}/", json={{}})
    if create_resp.status_code in (200, 201):
        item_id = create_resp.json().get("id")
        update_resp = client.put(f"/{mn}/{{item_id}}", json={{}})
        assert update_resp.status_code in (200, 422)


def test_delete_{mn}(client: TestClient):
    create_resp = client.post("/{mn}/", json={{}})
    if create_resp.status_code in (200, 201):
        item_id = create_resp.json().get("id")
        del_resp = client.delete(f"/{mn}/{{item_id}}")
        assert del_resp.status_code in (200, 204)
"#,
        mn = mn,
    )
}

// ── README.md ─────────────────────────────────────────────────────────────────

fn generate_readme(project: &Project, _output_dir: &str) -> String {
    let model_list: String = project
        .models
        .iter()
        .map(|m| format!("- **{}** ({} fields)\n", m.name, m.fields.len()))
        .collect();

    let endpoint_list: String = project
        .endpoints
        .iter()
        .map(|e| format!("- `{} {}` — {}\n", e.method.as_str(), e.path, e.description))
        .collect();

    let auth_section = if project.auth_config.enabled {
        format!(
            r#"
## 🔐 Authentication

Strategy: **{}**

Pre-generated endpoints:
- `POST /auth/register` — Register new user
- `POST /auth/login` — Get JWT token
- `GET /auth/me` — Get current user (requires Bearer token)
- `POST /auth/refresh` — Refresh token
- `POST /auth/logout` — Logout

**Usage:**
```bash
# Login
curl -X POST /auth/login -d "username=admin&password=secret"

# Use token
curl -H "Authorization: Bearer <token>" /auth/me
```
"#,
            project.auth_config.strategy.as_str()
        )
    } else {
        String::new()
    };

    format!(
        r#"# {name} — Generated by BackForge

A production-ready FastAPI backend.

## 📁 Project Structure

```
{name_lower}/
├── main.py              # FastAPI app entry point
├── requirements.txt     # Python dependencies
├── .env.example         # Environment variables template
├── Dockerfile
├── docker-compose.yml
├── app/
│   ├── models/          # SQLAlchemy ORM models
│   ├── schemas/         # Pydantic request/response schemas
│   ├── routers/         # FastAPI route handlers
│   ├── auth/            # JWT authentication
│   ├── db/              # Database engine & session
│   └── core/            # Config / settings
└── tests/               # pytest test suite
```

## 🚀 Quick Start

```bash
# 1. Copy env file
cp .env.example .env

# 2. Install dependencies
pip install -r requirements.txt

# 3. Run the server
uvicorn main:app --reload

# 4. Open API docs
open http://localhost:8000/docs
```

## 🗄️ Connecting a Real Database

Edit your `.env` file and change `DATABASE_URL`:

### PostgreSQL
```
DATABASE_URL=postgresql://user:password@localhost:5432/{db_name}
```
Install driver: `pip install psycopg2-binary`

### MySQL
```
DATABASE_URL=mysql+pymysql://user:password@localhost:3306/{db_name}
```
Install driver: `pip install pymysql`

### SQLite (default, no setup needed)
```
DATABASE_URL=sqlite:///./app.db
```

> ⚠️ **Important:** The fake tables used during BackForge design are for visual prototyping only.
> After changing `DATABASE_URL`, run `alembic` migrations or call `Base.metadata.create_all()` to create real tables.

### Alembic Migrations (recommended for production)
```bash
pip install alembic
alembic init migrations
# Edit migrations/env.py to import your Base
alembic revision --autogenerate -m "initial"
alembic upgrade head
```

## 📦 Models

{model_list}

## 🛣️ Endpoints

{endpoint_list}
{auth_section}
## 🧪 Running Tests

```bash
pytest tests/ -v
```

## 🐳 Docker

```bash
docker-compose up --build
```
API will be available at `http://localhost:8000`

---
*Generated with ❤️ by BackForge*
"#,
        name = project.name,
        name_lower = project.name.to_lowercase().replace(' ', "_"),
        db_name = project.name.to_lowercase().replace(' ', "_"),
        model_list = model_list,
        endpoint_list = endpoint_list,
        auth_section = auth_section,
    )
}

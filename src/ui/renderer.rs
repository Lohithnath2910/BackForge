use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Paragraph,
        Row, Table, Tabs, Wrap,
    },
    Frame,
};

use crate::ui::state::{AppState, EndpointForm, Modal, Screen, ServerStatus};
use crate::core::models::*;

// ─── Color Palette ────────────────────────────────────────────────────────────
const COL_ACCENT: Color = Color::Rgb(99, 179, 237);   // sky blue
const COL_GREEN: Color = Color::Rgb(72, 199, 142);
const COL_YELLOW: Color = Color::Rgb(255, 212, 59);
const COL_RED: Color = Color::Rgb(252, 100, 100);
const COL_MAGENTA: Color = Color::Rgb(217, 119, 226);
const COL_CYAN: Color = Color::Rgb(102, 217, 239);
const COL_DIM: Color = Color::Rgb(100, 100, 120);
const COL_BG_DARK: Color = Color::Rgb(18, 18, 28);
const COL_BG_PANEL: Color = Color::Rgb(26, 26, 40);
const COL_BORDER: Color = Color::Rgb(60, 60, 90);
const COL_TEXT: Color = Color::Rgb(220, 220, 240);

fn method_color(method: &HttpMethod) -> Color {
    match method {
        HttpMethod::GET => COL_GREEN,
        HttpMethod::POST => COL_ACCENT,
        HttpMethod::PUT => COL_YELLOW,
        HttpMethod::PATCH => COL_MAGENTA,
        HttpMethod::DELETE => COL_RED,
        HttpMethod::WebSocket => COL_CYAN,
    }
}

fn block(title: &str, focused: bool) -> Block<'_> {
    let border_color = if focused { COL_ACCENT } else { COL_BORDER };
    Block::default()
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(COL_BG_PANEL))
}

// ─── Main Render Dispatch ─────────────────────────────────────────────────────

pub fn render(f: &mut Frame, state: &AppState) {
    // Background fill
    let bg = Block::default().style(Style::default().bg(COL_BG_DARK));
    f.render_widget(bg, f.area());

    match &state.screen {
        Screen::Home => render_home(f, state),
        Screen::ModelList => render_model_list(f, state),
        Screen::ModelEditor => render_model_editor(f, state),
        Screen::EndpointList => render_endpoint_list(f, state),
        Screen::EndpointEditor => render_endpoint_editor(f, state),
        Screen::AuthSetup => render_auth_setup(f, state),
        Screen::FakeDbViewer => render_fake_db(f, state),
        Screen::ServerRunner => render_server_runner(f, state),
        Screen::EndpointTester => render_endpoint_tester(f, state),
        Screen::ExportPanel => render_export(f, state),
        Screen::Help => render_help(f, state),
    }

    // Modals overlay on top
    match &state.modal {
        Modal::None => {}
        Modal::NewModel => render_new_model_modal(f, state),
        Modal::NewField => render_new_field_modal(f, state),
        Modal::NewEndpoint => render_new_endpoint_modal(f, state),
        Modal::ConfirmDelete(id) => render_confirm_delete(f, id),
        Modal::AuthEnable => render_auth_enable_modal(f, state),
    }

    // Notification toast
    if let Some((msg, _)) = &state.notification {
        render_toast(f, msg);
    }
}

// ─── Home Screen ──────────────────────────────────────────────────────────────

fn render_home(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    // ASCII logo
    let logo = vec![
        Line::from(vec![
            Span::styled("  ██████╗  █████╗  ██████╗██╗  ██╗███████╗ ██████╗ ██████╗  ██████╗ ███████╗", Style::default().fg(COL_ACCENT)),
        ]),
        Line::from(vec![
            Span::styled("  ██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝", Style::default().fg(COL_ACCENT)),
        ]),
        Line::from(vec![
            Span::styled("  ██████╔╝███████║██║     █████╔╝ █████╗  ██║   ██║██████╔╝██║  ███╗█████╗  ", Style::default().fg(Color::Rgb(150, 200, 255))),
        ]),
        Line::from(vec![
            Span::styled("  ██╔══██╗██╔══██║██║     ██╔═██╗ ██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝  ", Style::default().fg(Color::Rgb(150, 200, 255))),
        ]),
        Line::from(vec![
            Span::styled("  ██████╔╝██║  ██║╚██████╗██║  ██╗██║     ╚██████╔╝██║  ██║╚██████╔╝███████╗", Style::default().fg(COL_DIM)),
        ]),
    ];

    let logo_para = Paragraph::new(logo)
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center);
    f.render_widget(logo_para, chunks[0]);

    // Menu cards
    let menu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(chunks[1]);

    let models_str = state.project.models.len().to_string();
    let endpoints_str = state.project.endpoints.len().to_string();
    let auth_str = if state.project.auth_config.enabled { "ON".to_string() } else { "OFF".to_string() };
    let server_str = if state.server_status != ServerStatus::Stopped { "RUNNING".to_string() } else { "STOPPED".to_string() };

    let left_items: Vec<(&str, &str, &str, Color)> = vec![
        ("m", "Models",    models_str.as_str(),    COL_GREEN),
        ("e", "Endpoints", endpoints_str.as_str(), COL_ACCENT),
        ("a", "Auth",      auth_str.as_str(),       COL_YELLOW),
    ];

    let right_items: Vec<(&str, &str, &str, Color)> = vec![
        ("d", "Fake DB Viewer", "",                  COL_CYAN),
        ("s", "Server Runner",  server_str.as_str(), COL_GREEN),
        ("x", "Export Project", "",                  COL_MAGENTA),
    ];

    let make_menu = |items: &[(&str, &str, &str, Color)]| -> Vec<Line<'static>> {
        items
            .iter()
            .map(|(key, label, val, color)| {
                Line::from(vec![
                    Span::styled(
                        format!(" [{}] ", key),
                        Style::default().fg(*color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<20}", label),
                        Style::default().fg(COL_TEXT),
                    ),
                    Span::styled(
                        val.to_string(),
                        Style::default().fg(*color).add_modifier(Modifier::DIM),
                    ),
                ])
            })
            .collect()
    };

    let left_para = Paragraph::new(make_menu(&left_items))
        .block(block("Build", false))
        .alignment(Alignment::Left);
    f.render_widget(left_para, menu_chunks[0]);

    let right_para = Paragraph::new(make_menu(&right_items))
        .block(block("Run & Export", false))
        .alignment(Alignment::Left);
    f.render_widget(right_para, menu_chunks[1]);

    // Project info
    let models_count = state.project.models.len();
    let endpoints_count = state.project.endpoints.len();
    let auth_status = if state.project.auth_config.enabled {
        format!("✓ {}", state.project.auth_config.strategy.as_str())
    } else {
        "✗ Disabled".to_string()
    };
    let info_text = vec![
        Line::from(vec![
            Span::styled("  Project: ", Style::default().fg(COL_DIM)),
            Span::styled(state.project.name.clone(), Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} Models  ", models_count), Style::default().fg(COL_GREEN)),
            Span::styled(format!("{} Endpoints  ", endpoints_count), Style::default().fg(COL_ACCENT)),
            Span::styled(format!("Auth: {}", auth_status), Style::default().fg(COL_YELLOW)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [h] Help   [q] Quit", Style::default().fg(COL_DIM)),
        ]),
    ];
    let info_para = Paragraph::new(info_text)
        .block(block("Project Status", false));
    f.render_widget(info_para, menu_chunks[2]);

    // Status bar
    let status = Line::from(vec![
        Span::styled(" BackForge v0.1 ", Style::default().fg(COL_BG_DARK).bg(COL_ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("  Rust + Ratatui TUI Backend Playground  ", Style::default().fg(COL_DIM)),
        Span::styled("Press [?] for help", Style::default().fg(COL_DIM)),
    ]);
    let status_bar = Paragraph::new(status)
        .style(Style::default().bg(Color::Rgb(20, 20, 35)));
    f.render_widget(status_bar, chunks[2]);
}

// ─── Model List Screen ────────────────────────────────────────────────────────

fn render_model_list(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let show_er = state.show_er_diagram && !state.project.models.is_empty();

    let chunks = if show_er {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    };

    // Model list panel
    let items: Vec<ListItem> = state
        .project
        .models
        .iter()
        .map(|m| {
            let field_count = m.fields.len();
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", m.name),
                    Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("({} fields)", field_count),
                    Style::default().fg(COL_DIM),
                ),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    if !state.project.models.is_empty() {
        list_state.select(Some(state.selected_model_idx));
    }

    let list = List::new(items)
        .block(block("Models  [n]New  [r]Rename  [Enter]Fields  [d]Delete  [v]ER  [q]Back", true))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 50, 80))
                .fg(COL_ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[0], &mut list_state);

    // ER Diagram panel
    if show_er {
        render_er_diagram(f, state, chunks[1]);
    }
}

// ─── ER Diagram ───────────────────────────────────────────────────────────────

fn render_er_diagram(f: &mut Frame, state: &AppState, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for model in &state.project.models {
        let is_selected = state.project.models.iter().position(|m| m.id == model.id)
            == Some(state.selected_model_idx);

        let header_style = if is_selected {
            Style::default().fg(COL_BG_DARK).bg(COL_ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD)
        };

        // Top border
        lines.push(Line::from(Span::styled(
            format!("  ┌─ {} {}", model.name, "─".repeat(20_usize.saturating_sub(model.name.len()))),
            header_style,
        )));

        for field in &model.fields {
            let pk_marker = if field.primary_key { "🔑" } else { "  " };
            let null_marker = if !field.nullable { "!" } else { " " };
            let uniq_marker = if field.unique { "U" } else { " " };

            lines.push(Line::from(vec![
                Span::styled("  │  ", Style::default().fg(COL_BORDER)),
                Span::styled(pk_marker, Style::default()),
                Span::styled(
                    format!(" {:<18}", field.name),
                    Style::default().fg(COL_TEXT),
                ),
                Span::styled(
                    format!("{:<12}", field.data_type.as_str()),
                    Style::default().fg(COL_YELLOW),
                ),
                Span::styled(
                    format!("{}{}", null_marker, uniq_marker),
                    Style::default().fg(COL_DIM),
                ),
            ]));
        }
        lines.push(Line::from(Span::styled(
            "  └─────────────────────────────────",
            Style::default().fg(COL_BORDER),
        )));
        lines.push(Line::from(""));
    }

    let para = Paragraph::new(lines)
        .block(block("ER Diagram", false))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

// ─── Model Editor Screen ──────────────────────────────────────────────────────

fn render_model_editor(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let Some(model) = state.project.models.get(state.selected_model_idx) else {
        return;
    };

    // Fields table
    let header = Row::new(vec!["", "Field Name", "Type", "Null", "Unique", "PK"])
        .style(Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD))
        .height(1);

    let rows: Vec<Row> = model
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let sel = i == state.selected_field_idx;
            let sel_style = if sel {
                Style::default().bg(Color::Rgb(40, 50, 80)).fg(COL_ACCENT)
            } else {
                Style::default().fg(COL_TEXT)
            };
            Row::new(vec![
                Cell::from(if sel { "▶" } else { " " }).style(sel_style),
                Cell::from(f.name.clone()).style(sel_style),
                Cell::from(f.data_type.as_str()).style(Style::default().fg(COL_YELLOW)),
                Cell::from(if f.nullable { "✓" } else { "✗" })
                    .style(Style::default().fg(if f.nullable { COL_GREEN } else { COL_RED })),
                Cell::from(if f.unique { "✓" } else { "✗" })
                    .style(Style::default().fg(if f.unique { COL_GREEN } else { COL_DIM })),
                Cell::from(if f.primary_key { "🔑" } else { " " }),
            ])
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Percentage(35),
            Constraint::Percentage(20),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(4),
        ],
    )
    .header(header);

    let model_title = format!("Model: {}  [n]Add Field  [d]Delete  [u]Toggle Null  [q]Back", model.name);
    let table = table.block(block(&model_title, true));

    f.render_widget(table, chunks[0]);

    // Live code preview
    let code = generate_model_preview(model);
    let preview = Paragraph::new(code)
        .block(block("FastAPI / SQLAlchemy Preview", false))
        .wrap(Wrap { trim: false });
    f.render_widget(preview, chunks[1]);
}

fn generate_model_preview(model: &Model) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    let kw = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Rgb(197, 134, 192)));
    let cls = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Rgb(78, 201, 176)));
    let typ = |s: &str| Span::styled(s.to_string(), Style::default().fg(COL_YELLOW));
    let txt = |s: &str| Span::styled(s.to_string(), Style::default().fg(COL_TEXT));
    let cmt = |s: &str| Span::styled(s.to_string(), Style::default().fg(COL_DIM));

    lines.push(Line::from(cmt("# ─── SQLAlchemy Model ───────────────")));
    lines.push(Line::from(vec![kw("class "), cls(&model.name), txt("(Base):")]));
    lines.push(Line::from(vec![txt("    __tablename__ = "), Span::styled(format!("\"{}\"", model.name.to_lowercase()), Style::default().fg(Color::Rgb(206, 145, 120)))]));
    lines.push(Line::from(""));

    for f in &model.fields {
        let col_args = if f.primary_key {
            ", primary_key=True".to_string()
        } else {
            let mut args = String::new();
            if !f.nullable { args.push_str(", nullable=False"); }
            if f.unique { args.push_str(", unique=True"); }
            args
        };
        lines.push(Line::from(vec![
            txt("    "),
            Span::styled(f.name.clone(), Style::default().fg(COL_ACCENT)),
            txt(" = Column("),
            typ(f.data_type.to_sqlalchemy_type()),
            txt(&col_args),
            txt(")"),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(cmt("# ─── Pydantic Schema ────────────────")));
    lines.push(Line::from(vec![kw("class "), cls(&format!("{}Base", model.name)), txt("(BaseModel):")]));
    for f in model.fields.iter().filter(|f| !f.primary_key) {
        let opt = if f.nullable {
            format!("Optional[{}]", f.data_type.to_python_type())
        } else {
            f.data_type.to_python_type().to_string()
        };
        lines.push(Line::from(vec![
            txt("    "),
            Span::styled(f.name.clone(), Style::default().fg(COL_ACCENT)),
            txt(": "),
            typ(&opt),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![kw("class "), cls(&format!("{}Response", model.name)), txt(&format!("({}Base):", model.name))]));
    lines.push(Line::from(vec![
        txt("    "),
        Span::styled("id", Style::default().fg(COL_ACCENT)),
        txt(": "),
        typ("int"),
    ]));
    lines.push(Line::from(vec![
        txt("    "),
        kw("class "),
        cls("Config"),
        txt(":"),
    ]));
    lines.push(Line::from(vec![txt("        from_attributes = "), Span::styled("True", Style::default().fg(Color::Rgb(86, 156, 214)))]));

    lines
}

// ─── Endpoint List ────────────────────────────────────────────────────────────

fn render_endpoint_list(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let header = Row::new(vec!["", "Method", "Path", "CRUD Op", "Model", "Auth", "Tags"])
        .style(Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = state
        .project
        .endpoints
        .iter()
        .enumerate()
        .map(|(i, ep)| {
            let sel = i == state.selected_endpoint_idx;
            let sel_bg = if sel { Color::Rgb(40, 50, 80) } else { COL_BG_PANEL };
            let model_name = ep
                .linked_model
                .as_ref()
                .and_then(|id| state.project.get_model_by_id(id))
                .map(|m| m.name.clone())
                .unwrap_or_else(|| "-".to_string());

            Row::new(vec![
                Cell::from(if sel { "▶" } else { " " }),
                Cell::from(ep.method.as_str()).style(
                    Style::default()
                        .fg(method_color(&ep.method))
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(ep.path.clone()).style(Style::default().fg(COL_TEXT)),
                Cell::from(ep.crud_op.as_str()).style(Style::default().fg(COL_DIM)),
                Cell::from(model_name).style(Style::default().fg(COL_YELLOW)),
                Cell::from(if ep.requires_auth { "🔒" } else { "  " }),
                Cell::from(ep.tags.join(",")).style(Style::default().fg(COL_DIM)),
            ])
            .style(Style::default().bg(sel_bg))
        })
        .collect();

    let mut list_state = ListState::default();
    if !state.project.endpoints.is_empty() {
        list_state.select(Some(state.selected_endpoint_idx));
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(8),
            Constraint::Percentage(30),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(5),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .block(block(
        "Endpoints  [n]New  [Enter]Edit  [t]Test  [d]Delete  [q]Back",
        true,
    ));

    f.render_widget(table, chunks[0]);

    // Hint bar
    let hint = Paragraph::new(Line::from(vec![
        Span::styled(" [n]", Style::default().fg(COL_GREEN)),
        Span::styled(" New   ", Style::default().fg(COL_DIM)),
        Span::styled("[t]", Style::default().fg(COL_ACCENT)),
        Span::styled(" Test Endpoint   ", Style::default().fg(COL_DIM)),
        Span::styled("[d]", Style::default().fg(COL_RED)),
        Span::styled(" Delete   ", Style::default().fg(COL_DIM)),
        Span::styled("[Enter]", Style::default().fg(COL_YELLOW)),
        Span::styled(" Details   ", Style::default().fg(COL_DIM)),
        Span::styled("[q]", Style::default().fg(COL_DIM)),
        Span::styled(" Back", Style::default().fg(COL_DIM)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(COL_BORDER)));
    f.render_widget(hint, chunks[1]);
}

// ─── Endpoint Editor ──────────────────────────────────────────────────────────

fn render_endpoint_editor(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let Some(ep) = state.project.endpoints.get(state.selected_endpoint_idx) else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Details panel
    let model_name = ep
        .linked_model
        .as_ref()
        .and_then(|id| state.project.get_model_by_id(id))
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "None".to_string());

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Path:     ", Style::default().fg(COL_DIM)),
            Span::styled(ep.path.clone(), Style::default().fg(COL_TEXT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Method:   ", Style::default().fg(COL_DIM)),
            Span::styled(ep.method.as_str().to_string(), Style::default().fg(method_color(&ep.method)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  CRUD:     ", Style::default().fg(COL_DIM)),
            Span::styled(ep.crud_op.as_str().to_string(), Style::default().fg(COL_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Model:    ", Style::default().fg(COL_DIM)),
            Span::styled(model_name, Style::default().fg(COL_YELLOW)),
        ]),
        Line::from(vec![
            Span::styled("  Auth:     ", Style::default().fg(COL_DIM)),
            Span::styled(
                if ep.requires_auth { "🔒 Required" } else { "🔓 Public" }.to_string(),
                Style::default().fg(if ep.requires_auth { COL_YELLOW } else { COL_GREEN }),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Desc:     ", Style::default().fg(COL_DIM)),
            Span::styled(ep.description.clone(), Style::default().fg(COL_TEXT)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Body Params:", Style::default().fg(COL_DIM))),
    ];

    let mut detail_lines = lines;
    for (k, v) in &ep.body_params {
        detail_lines.push(Line::from(vec![
            Span::styled("    • ", Style::default().fg(COL_ACCENT)),
            Span::styled(k.clone(), Style::default().fg(COL_TEXT)),
            Span::styled(": ", Style::default().fg(COL_DIM)),
            Span::styled(v.clone(), Style::default().fg(COL_YELLOW)),
        ]));
    }
    if ep.body_params.is_empty() {
        detail_lines.push(Line::from(Span::styled("    (none)", Style::default().fg(COL_DIM))));
    }

    detail_lines.push(Line::from(""));
    detail_lines.push(Line::from(Span::styled(
        "  [e] Edit endpoint   [a] Toggle Auth   [q] Back",
        Style::default().fg(COL_DIM),
    )));

    let details = Paragraph::new(detail_lines)
        .block(block("Endpoint Details  [e] Edit  [a] Toggle Auth  [q] Back", true))
        .wrap(Wrap { trim: false });
    f.render_widget(details, chunks[0]);

    // Code preview
    let code = generate_endpoint_preview(ep, state);
    let preview = Paragraph::new(code)
        .block(block("FastAPI Route Preview", false))
        .wrap(Wrap { trim: false });
    f.render_widget(preview, chunks[1]);
}

fn generate_endpoint_preview(ep: &Endpoint, state: &AppState) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    let kw = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Rgb(197, 134, 192)));
    let dec = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Rgb(220, 220, 100)));
    let txt = |s: &str| Span::styled(s.to_string(), Style::default().fg(COL_TEXT));
    let typ = |s: &str| Span::styled(s.to_string(), Style::default().fg(COL_YELLOW));
    let cmt = |s: &str| Span::styled(s.to_string(), Style::default().fg(COL_DIM));
    let str_lit = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Rgb(206, 145, 120)));

    let method_lower = ep.method.as_str().to_lowercase();
    let router_dec = format!("@router.{}(\"{}\"", method_lower, ep.path);
    let auth_dep = if ep.requires_auth { ", dependencies=[Depends(get_current_user)]" } else { "" };

    lines.push(Line::from(cmt(&format!("# {}", ep.description))));
    lines.push(Line::from(vec![dec(&router_dec), txt(auth_dep), txt(")")]));

    // Function signature
    let model_name = ep.linked_model.as_ref()
        .and_then(|id| state.project.get_model_by_id(id))
        .map(|m| m.name.clone());

    let mut params: Vec<String> = Vec::new();
    for p in &ep.path_params {
        params.push(format!("{}: int", p));
    }
    if matches!(ep.method, HttpMethod::POST | HttpMethod::PUT | HttpMethod::PATCH) {
        if let Some(ref mn) = model_name {
            params.push(format!("data: {}Base", mn));
        } else if !ep.body_params.is_empty() {
            params.push("data: dict".to_string());
        }
    }
    if ep.requires_auth {
        params.push("current_user: User = Depends(get_current_user)".to_string());
    }
    params.push("db: Session = Depends(get_db)".to_string());

    let return_type = if let Some(ref mn) = model_name {
        match ep.crud_op {
            CrudOp::ReadAll => format!("List[{}Response]", mn),
            _ => format!("{}Response", mn),
        }
    } else {
        "dict".to_string()
    };

    lines.push(Line::from(vec![
        kw("async def "),
        Span::styled(
            format!("{}_{}", method_lower, ep.path.trim_matches('/').replace('/', "_").replace('{', "").replace('}', "")),
            Style::default().fg(Color::Rgb(220, 220, 100)),
        ),
        txt("("),
    ]));
    for p in &params {
        lines.push(Line::from(vec![txt("    "), typ(p), txt(",")]));
    }
    lines.push(Line::from(vec![txt(") -> "), typ(&return_type), txt(":")]));
    lines.push(Line::from(vec![txt("    "), cmt("# Auto-generated handler")]));

    // Body stub
    if let Some(mn) = &model_name {
        match ep.crud_op {
            CrudOp::Create => {
                lines.push(Line::from(vec![txt("    db_item = "), Span::styled(mn.clone(), Style::default().fg(Color::Rgb(78, 201, 176))), txt("(**data.dict())")]));
                lines.push(Line::from(vec![txt("    db.add(db_item)")]));
                lines.push(Line::from(vec![txt("    db.commit(); db.refresh(db_item)")]));
                lines.push(Line::from(vec![txt("    "), kw("return "), txt("db_item")]));
            }
            CrudOp::ReadAll => {
                lines.push(Line::from(vec![txt("    "), kw("return "), txt("db.query("), Span::styled(mn.clone(), Style::default().fg(Color::Rgb(78, 201, 176))), txt(").all()")]));
            }
            CrudOp::ReadOne => {
                lines.push(Line::from(vec![txt("    item = db.query("), Span::styled(mn.clone(), Style::default().fg(Color::Rgb(78, 201, 176))), txt(").filter(...).first()")]));
                lines.push(Line::from(vec![txt("    "), kw("if not "), txt("item: raise HTTPException(404)")]));
                lines.push(Line::from(vec![txt("    "), kw("return "), txt("item")]));
            }
            CrudOp::Delete => {
                lines.push(Line::from(vec![txt("    db.delete(item); db.commit()")]));
                lines.push(Line::from(vec![txt("    "), kw("return "), str_lit("{\"ok\": true}")]));
            }
            _ => {
                lines.push(Line::from(vec![txt("    "), kw("pass")]));
            }
        }
    } else {
        lines.push(Line::from(vec![txt("    "), kw("pass")]));
    }

    lines
}

// ─── Auth Setup ───────────────────────────────────────────────────────────────

fn render_auth_setup(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(5)])
        .split(area);

    let auth = &state.project.auth_config;
    let strategy_str = auth.strategy.as_str();

    let status_color = if auth.enabled { COL_GREEN } else { COL_RED };
    let status_text = if auth.enabled { "✓ ENABLED" } else { "✗ DISABLED" };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Status:    ", Style::default().fg(COL_DIM)),
            Span::styled(status_text, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Strategy:  ", Style::default().fg(COL_DIM)),
            Span::styled(strategy_str.to_string(), Style::default().fg(COL_ACCENT)),
            Span::styled("  (← → to change when enabled)", Style::default().fg(COL_DIM)),
        ]),
        Line::from(vec![
            Span::styled("  Expiry:    ", Style::default().fg(COL_DIM)),
            Span::styled(format!("{} minutes", auth.token_expiry_minutes), Style::default().fg(COL_YELLOW)),
        ]),
        Line::from(vec![
            Span::styled("  Refresh:   ", Style::default().fg(COL_DIM)),
            Span::styled(if auth.refresh_token { "✓ Yes" } else { "✗ No" }, Style::default().fg(COL_TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [e] Enable Auth   [d] Disable   [← →] Change Strategy   [q] Back",
                Style::default().fg(COL_DIM)),
        ]),
    ];

    let info = Paragraph::new(lines)
        .block(block("Auth Configuration", true))
        .wrap(Wrap { trim: false });
    f.render_widget(info, chunks[0]);

    // Auth endpoints list
    let auth_eps: Vec<ListItem> = state
        .project
        .endpoints
        .iter()
        .filter(|e| e.tags.contains(&"auth".to_string()))
        .map(|ep| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {:6}", ep.method.as_str()),
                    Style::default().fg(method_color(&ep.method)).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {:<25}", ep.path), Style::default().fg(COL_TEXT)),
                Span::styled(ep.description.clone(), Style::default().fg(COL_DIM)),
            ]))
        })
        .collect();

    let auth_list = List::new(auth_eps)
        .block(block("Auto-Generated Auth Endpoints", false));
    f.render_widget(auth_list, chunks[1]);
}

// ─── Fake DB Viewer ───────────────────────────────────────────────────────────

fn render_fake_db(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    if state.project.models.is_empty() {
        let empty = Paragraph::new("No models yet. Create some in the Models screen!")
            .block(block("Fake DB Viewer", true))
            .style(Style::default().fg(COL_DIM));
        f.render_widget(empty, area);
        return;
    }

    // Model tabs
    let tab_titles: Vec<Line> = state
        .project
        .models
        .iter()
        .map(|m| Line::from(m.name.clone()))
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(COL_BORDER)))
        .select(state.fake_db_model_idx)
        .highlight_style(
            Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" | ", Style::default().fg(COL_BORDER)));
    f.render_widget(tabs, chunks[0]);

    let Some(model) = state.project.models.get(state.fake_db_model_idx) else { return };
    let Some(table) = state.project.get_fake_table(&model.id) else {
        let empty_title = format!("{} — 0 rows  [← →] Switch table  [c] Clear  [q] Back", model.name);
        let empty = Paragraph::new(" No data in this table yet.")
            .block(block(&empty_title, false))
            .style(Style::default().fg(COL_DIM));
        f.render_widget(empty, chunks[1]);
        return;
    };

    // Build header from non-PK model fields (id always first)
    // Field names are lowercased to match SQLAlchemy columns and Pydantic schemas
    let non_pk_fields: Vec<String> = model.fields.iter()
        .filter(|f| !f.primary_key)
        .map(|f| f.name.to_lowercase())
        .collect();

    let mut header_cells = vec![
        Cell::from("id").style(Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD))
    ];
    for fname in &non_pk_fields {
        header_cells.push(Cell::from(fname.clone()).style(Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD)));
    }
    // If model has no defined fields, derive columns from first row's keys
    let header_names: Vec<String> = if non_pk_fields.is_empty() {
        table.rows.first()
            .map(|r| r.values.iter().map(|(k, _)| k.clone()).collect())
            .unwrap_or_default()
    } else {
        non_pk_fields.clone()
    };
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = table.rows.iter().map(|row| {
        let mut cells = vec![
            Cell::from(row.id.to_string()).style(Style::default().fg(COL_YELLOW))
        ];
        // Match each header column to its value by name (alignment-safe)
        for col in &header_names {
            let val = row.values.iter()
                .find(|(k, _)| k == col)
                .map(|(_, v)| v.as_str())
                .unwrap_or("—");
            cells.push(Cell::from(val.to_string()).style(Style::default().fg(COL_TEXT)));
        }
        Row::new(cells)
    }).collect();

    let col_count = (header_names.len() + 1).max(2); // +1 for id col
    let id_pct = 8u16;
    let rest_pct = (100u16 - id_pct) / (col_count as u16 - 1).max(1);
    let mut constraints = vec![Constraint::Percentage(id_pct)];
    for _ in &header_names {
        constraints.push(Constraint::Percentage(rest_pct));
    }

    let db_title = format!("{} — {} rows  [← →] Switch  [c] Clear  [q] Back", model.name, table.rows.len());
    let db_table = Table::new(rows, constraints)
        .header(header)
        .block(block(&db_title, true));
    f.render_widget(db_table, chunks[1]);
}

// ─── Server Runner ────────────────────────────────────────────────────────────

fn render_server_runner(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(5)])
        .split(area);

    let (status_color, status_text, status_icon) = match &state.server_status {
        ServerStatus::Stopped => (COL_RED, "STOPPED".to_string(), "⬛"),
        ServerStatus::Starting => (COL_YELLOW, "STARTING...".to_string(), "🔄"),
        ServerStatus::Running { port } => (COL_GREEN, format!("RUNNING on :{}", port), "🟢"),
        ServerStatus::Error(e) => (COL_RED, format!("ERROR: {}", e), "❌"),
    };

    let status_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(COL_DIM)),
            Span::styled(format!("{} {}", status_icon, status_text), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [s] Start Server   ", Style::default().fg(COL_GREEN)),
            Span::styled("[x] Stop   ", Style::default().fg(COL_RED)),
            Span::styled("[c] Clear logs   ", Style::default().fg(COL_DIM)),
            Span::styled("[q] Back", Style::default().fg(COL_DIM)),
        ]),
    ];

    let status_panel = Paragraph::new(status_lines)
        .block(block("Server Runner (uvicorn + FastAPI)", true));
    f.render_widget(status_panel, chunks[0]);

    // Server logs
    let log_lines: Vec<Line> = state
        .server_logs
        .iter()
        .map(|l| {
            let color = if l.contains("ERROR") || l.contains("error") {
                COL_RED
            } else if l.contains("INFO") || l.contains("started") {
                COL_GREEN
            } else if l.contains("WARNING") {
                COL_YELLOW
            } else {
                COL_TEXT
            };
            Line::from(Span::styled(format!("  {}", l), Style::default().fg(color)))
        })
        .collect();

    let logs = Paragraph::new(if log_lines.is_empty() {
        vec![Line::from(Span::styled("  Logs will appear here when server starts...", Style::default().fg(COL_DIM)))]
    } else {
        log_lines
    })
    .block(block("Logs", false));
    f.render_widget(logs, chunks[1]);
}

// ─── Export Panel ─────────────────────────────────────────────────────────────

fn render_export(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(5)])
        .split(area);

    let path_style = if state.export_path_editing {
        Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COL_TEXT)
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Output path:  ", Style::default().fg(COL_DIM)),
            Span::styled(state.export_path.clone(), path_style),
            if state.export_path_editing {
                Span::styled("█", Style::default().fg(COL_ACCENT))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Will generate:", Style::default().fg(COL_DIM)),
        ]),
        Line::from(vec![
            Span::styled("    app/  models/  schemas/  routers/  auth/  db/  tests/  main.py  requirements.txt  Dockerfile  README.md", Style::default().fg(COL_GREEN)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [e] Edit path   [Enter] Export Now   [q] Back",
                Style::default().fg(COL_DIM)),
        ]),
    ];

    let settings = Paragraph::new(lines)
        .block(block("Export FastAPI Project", true))
        .wrap(Wrap { trim: false });
    f.render_widget(settings, chunks[0]);

    // Summary
    let summary_lines = vec![
        Line::from(vec![
            Span::styled("  Models:    ", Style::default().fg(COL_DIM)),
            Span::styled(state.project.models.len().to_string(), Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  ({})", state.project.models.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(", ")), Style::default().fg(COL_DIM)),
        ]),
        Line::from(vec![
            Span::styled("  Endpoints: ", Style::default().fg(COL_DIM)),
            Span::styled(state.project.endpoints.len().to_string(), Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Auth:      ", Style::default().fg(COL_DIM)),
            Span::styled(
                if state.project.auth_config.enabled {
                    format!("✓ {}", state.project.auth_config.strategy.as_str())
                } else { "✗ None".to_string() },
                Style::default().fg(if state.project.auth_config.enabled { COL_GREEN } else { COL_DIM }),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tests:     ", Style::default().fg(COL_DIM)),
            Span::styled("✓ pytest files included", Style::default().fg(COL_GREEN)),
        ]),
        Line::from(vec![
            Span::styled("  Docker:    ", Style::default().fg(COL_DIM)),
            Span::styled("✓ Dockerfile + docker-compose.yml", Style::default().fg(COL_GREEN)),
        ]),
        Line::from(vec![
            Span::styled("  DB Guide:  ", Style::default().fg(COL_DIM)),
            Span::styled("✓ README with PostgreSQL / MySQL / SQLite setup", Style::default().fg(COL_GREEN)),
        ]),
    ];

    let summary = Paragraph::new(summary_lines)
        .block(block("Export Summary", false))
        .wrap(Wrap { trim: false });
    f.render_widget(summary, chunks[1]);
}

// ─── Help Screen ──────────────────────────────────────────────────────────────

fn render_help(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  BACKFORGE — Keyboard Reference", Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("  GLOBAL", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  Ctrl+C / q   ", Style::default().fg(COL_GREEN)), Span::styled("Quit", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  Esc          ", Style::default().fg(COL_GREEN)), Span::styled("Go back / cancel", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  h / ?        ", Style::default().fg(COL_GREEN)), Span::styled("This help screen", Style::default().fg(COL_TEXT))]),
        Line::from(""),
        Line::from(Span::styled("  HOME SCREEN", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  m            ", Style::default().fg(COL_GREEN)), Span::styled("Models", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  e            ", Style::default().fg(COL_GREEN)), Span::styled("Endpoints", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  a            ", Style::default().fg(COL_GREEN)), Span::styled("Auth Setup", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  d            ", Style::default().fg(COL_GREEN)), Span::styled("Fake DB Viewer", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  s            ", Style::default().fg(COL_GREEN)), Span::styled("Server Runner", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  x            ", Style::default().fg(COL_GREEN)), Span::styled("Export Project", Style::default().fg(COL_TEXT))]),
        Line::from(""),
        Line::from(Span::styled("  MODELS", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  n            ", Style::default().fg(COL_GREEN)), Span::styled("New model", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  Enter        ", Style::default().fg(COL_GREEN)), Span::styled("Open model editor", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  d            ", Style::default().fg(COL_GREEN)), Span::styled("Delete selected model", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  v            ", Style::default().fg(COL_GREEN)), Span::styled("Toggle ER diagram", Style::default().fg(COL_TEXT))]),
        Line::from(""),
        Line::from(Span::styled("  MODEL EDITOR", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  n            ", Style::default().fg(COL_GREEN)), Span::styled("Add new field", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  d            ", Style::default().fg(COL_GREEN)), Span::styled("Delete selected field (non-PK only)", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  u            ", Style::default().fg(COL_GREEN)), Span::styled("Toggle nullable on field", Style::default().fg(COL_TEXT))]),
        Line::from(""),
        Line::from(Span::styled("  ADD FIELD FORM", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  Tab / ↑↓     ", Style::default().fg(COL_GREEN)), Span::styled("Navigate fields", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  ← →          ", Style::default().fg(COL_GREEN)), Span::styled("Change type / toggle booleans", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  Space        ", Style::default().fg(COL_GREEN)), Span::styled("Toggle nullable/unique/pk", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  Enter        ", Style::default().fg(COL_GREEN)), Span::styled("Confirm field (on last row)", Style::default().fg(COL_TEXT))]),
        Line::from(""),
        Line::from(Span::styled("  ENDPOINTS", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  n            ", Style::default().fg(COL_GREEN)), Span::styled("New endpoint", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  t            ", Style::default().fg(COL_GREEN)), Span::styled("Test selected endpoint (opens server)", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  d            ", Style::default().fg(COL_GREEN)), Span::styled("Delete endpoint", Style::default().fg(COL_TEXT))]),
        Line::from(""),
        Line::from(Span::styled("  SERVER", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  s            ", Style::default().fg(COL_GREEN)), Span::styled("Start server (requires Python + uvicorn)", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  x            ", Style::default().fg(COL_GREEN)), Span::styled("Stop server", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  c            ", Style::default().fg(COL_GREEN)), Span::styled("Clear logs", Style::default().fg(COL_TEXT))]),
        Line::from(""),
        Line::from(Span::styled("  EXPORT", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  e            ", Style::default().fg(COL_GREEN)), Span::styled("Edit output path", Style::default().fg(COL_TEXT))]),
        Line::from(vec![Span::styled("  Enter        ", Style::default().fg(COL_GREEN)), Span::styled("Generate full FastAPI project", Style::default().fg(COL_TEXT))]),
        Line::from(""),
        Line::from(Span::styled("  j/k or ↑↓ = navigate   [q] Back", Style::default().fg(COL_DIM))),
    ];

    let visible_height = area.height as usize - 2;
    let scroll = state.help_scroll.min(lines.len().saturating_sub(visible_height));

    let help = Paragraph::new(lines)
        .block(block("Help & Keyboard Shortcuts  [j/k] Scroll  [q] Back", true))
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(help, area);
}

// ─── Modals ───────────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let w = area.width * percent_x / 100;
    let x = area.x + (area.width - w) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, w, height.min(area.height))
}

fn render_new_model_modal(f: &mut Frame, state: &AppState) {
    let area = centered_rect(50, 7, f.area());
    f.render_widget(Clear, area);

    let title = if state.editing_model_id.is_some() { "Rename Model" } else { "New Model" };
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name: ", Style::default().fg(COL_DIM)),
            Span::styled(state.model_form.name.clone(), Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(COL_ACCENT)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  [Enter] Confirm   [Esc] Cancel", Style::default().fg(COL_DIM))),
    ];

    let modal = Paragraph::new(lines)
        .block(block(title, true))
        .wrap(Wrap { trim: false });
    f.render_widget(modal, area);
}

fn render_new_field_modal(f: &mut Frame, state: &AppState) {
    let area = centered_rect(60, 14, f.area());
    f.render_widget(Clear, area);

    let ff = state.field_form.focused_field;
    let types = DataType::variants();
    let current_type = &types[state.field_form.data_type_index];

    let focused = |idx: usize, label: &str, value: &str| -> Line<'static> {
        let (lbl_style, val_style) = if ff == idx {
            (
                Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD),
                Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(COL_DIM),
                Style::default().fg(COL_TEXT),
            )
        };
        let cursor = if ff == idx { "█" } else { "" };
        Line::from(vec![
            Span::styled(format!("  {:12}", label), lbl_style),
            Span::styled(value.to_string(), val_style),
            Span::styled(cursor, Style::default().fg(COL_ACCENT)),
        ])
    };

    let null_val = if state.field_form.nullable { "[✓] Yes" } else { "[ ] No" };
    let uniq_val = if state.field_form.unique { "[✓] Yes" } else { "[ ] No" };
    let pk_val = if state.field_form.primary_key { "[✓] Yes" } else { "[ ] No" };

    let lines = vec![
        Line::from(""),
        focused(0, "Field Name:", &state.field_form.name),
        Line::from(""),
        focused(1, "Data Type:", &format!("◀ {} ▶", current_type.as_str())),
        Line::from(""),
        focused(2, "Nullable:", null_val),
        focused(3, "Unique:", uniq_val),
        focused(4, "Primary Key:", pk_val),
        Line::from(""),
        Line::from(Span::styled("  [Tab]↓  [Shift+Tab]↑  [← →] Toggle  [Space] Toggle bool  [Enter on PK row] Confirm", Style::default().fg(COL_DIM))),
    ];

    let modal = Paragraph::new(lines)
        .block(block("Add Field", true))
        .wrap(Wrap { trim: false });
    f.render_widget(modal, area);
}

fn render_new_endpoint_modal(f: &mut Frame, state: &AppState) {
    let area = centered_rect(70, 26, f.area());
    f.render_widget(Clear, area);

    let ff = state.endpoint_form.focused_field;
    let methods = HttpMethod::variants();
    let crud_ops = ["Create", "Read One", "Read All", "Update", "Delete", "Custom"];
    let method = &methods[state.endpoint_form.method_index];
    let crud   = crud_ops[state.endpoint_form.crud_op_index];
    let auth_val = if state.endpoint_form.requires_auth { "[✓] Yes" } else { "[ ] No" };

    let model_val = if state.endpoint_form.linked_model_index == 0 {
        "None".to_string()
    } else {
        state.project.models.get(state.endpoint_form.linked_model_index - 1)
            .map(|m| m.name.clone()).unwrap_or_else(|| "None".to_string())
    };

    // Compute full URL preview
    let prefix = if state.endpoint_form.linked_model_index > 0 {
        state.project.models.get(state.endpoint_form.linked_model_index - 1)
            .map(|m| format!("/{}", m.name.to_lowercase().replace(' ', "_")))
            .unwrap_or_default()
    } else { String::new() };
    let path_display = if state.endpoint_form.path == "/" { String::new() }
        else { state.endpoint_form.path.clone() };
    let full_url = format!("http://127.0.0.1:8000{}{}", prefix, path_display);

    let row = |idx: usize, label: &str, value: &str| -> Line<'static> {
        let is_focused = ff == idx;
        let lbl = if is_focused {
            Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD)
        } else { Style::default().fg(COL_DIM) };
        let val = if is_focused { Style::default().fg(COL_ACCENT) }
                  else { Style::default().fg(COL_TEXT) };
        let cursor = if is_focused && matches!(idx, 0 | 6) { "█" } else { "" };
        Line::from(vec![
            Span::styled(format!("  {:16}", label), lbl),
            Span::styled(format!("{}{}", value, cursor), val),
        ])
    };

    // Get model fields for picker display
    let model_fields: Vec<String> = if state.endpoint_form.linked_model_index > 0 {
        state.project.models.get(state.endpoint_form.linked_model_index - 1)
            .map(|m| m.fields.iter().filter(|f| !f.primary_key).map(|f| f.name.clone()).collect())
            .unwrap_or_default()
    } else { Vec::new() };

    let uses_body = EndpointForm::method_uses_body(state.endpoint_form.method_index);
    let params_label = EndpointForm::params_section_label(state.endpoint_form.method_index);

    let mut lines = vec![
        Line::from(""),
        row(0, "Path:", &state.endpoint_form.path),
        // URL preview line
        Line::from(vec![
            Span::styled("                  → ", Style::default().fg(COL_DIM)),
            Span::styled(method.as_str().to_string(), Style::default().fg(method_color(method)).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {}", full_url), Style::default().fg(Color::Rgb(100, 160, 255))),
        ]),
        Line::from(""),
        row(1, "HTTP Method:", &format!("◀ {} ▶", method.as_str())),
        row(2, "CRUD Op:", &format!("◀ {} ▶  (auto-suggested, ← → to change)", crud)),
        row(3, "Requires Auth:", auth_val),
        row(4, "Linked Model:", &format!("◀ {} ▶", model_val)),
        Line::from(""),
        Line::from(Span::styled(format!("  {}", params_label), Style::default().fg(COL_DIM))),
    ];

    if model_fields.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Link a model above to pick fields, or leave empty",
            Style::default().fg(Color::Rgb(80, 80, 100))
        )));
    } else if !uses_body {
        lines.push(Line::from(Span::styled(
            "  Leave empty for ID-based ops (use {id} in path). Or pick fields to filter by:",
            Style::default().fg(Color::Rgb(80, 80, 100))
        )));
    }

    // Field picker row
    if !model_fields.is_empty() {
        let picker_focused = ff == 5;
        let picker_idx = state.endpoint_form.field_picker_idx % model_fields.len();

        // Show all fields as toggleable chips
        let mut chip_spans = vec![
            Span::styled(if picker_focused { "  ▶  " } else { "     " },
                Style::default().fg(COL_ACCENT)),
        ];
        for (i, fname) in model_fields.iter().enumerate() {
            let is_added = state.endpoint_form.body_params.iter().any(|k| k == fname);
            let is_cursor = picker_focused && i == picker_idx;
            let style = if is_added && is_cursor {
                Style::default().fg(COL_BG_DARK).bg(COL_GREEN).add_modifier(Modifier::BOLD)
            } else if is_added {
                Style::default().fg(COL_GREEN)
            } else if is_cursor {
                Style::default().fg(COL_BG_DARK).bg(COL_ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(COL_DIM)
            };
            chip_spans.push(Span::styled(format!(" [{}] ", fname), style));
        }
        lines.push(Line::from(chip_spans));
        if picker_focused {
            lines.push(Line::from(Span::styled(
                "  ← → cycle fields   Space/Enter = toggle field   Backspace = remove last",
                Style::default().fg(COL_DIM)
            )));
        }
    }

    // Show selected fields
    if !state.endpoint_form.body_params.is_empty() {
        let selected: Vec<String> = state.endpoint_form.body_params.clone();
        lines.push(Line::from(vec![
            Span::styled("  Selected: ", Style::default().fg(COL_DIM)),
            Span::styled(selected.join(", "), Style::default().fg(COL_GREEN)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(row(6, "Note (optional):", &state.endpoint_form.description));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Tab/↓] Next   [← →] Cycle/Navigate   [Enter on Note] Confirm   [Esc] Cancel",
        Style::default().fg(COL_DIM)
    )));

    let modal_title = if state.editing_endpoint_id.is_some() { "Edit Endpoint" } else { "New Endpoint" };
    let modal = Paragraph::new(lines)
        .block(block(modal_title, true))
        .wrap(Wrap { trim: false });
    f.render_widget(modal, area);
}

fn render_confirm_delete(f: &mut Frame, _id: &str) {
    let area = centered_rect(40, 7, f.area());
    f.render_widget(Clear, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Are you sure you want to delete this?", Style::default().fg(COL_TEXT))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [Y] Yes, delete   ", Style::default().fg(COL_RED).add_modifier(Modifier::BOLD)),
            Span::styled("[N / Esc] Cancel", Style::default().fg(COL_DIM)),
        ]),
    ];

    let modal = Paragraph::new(lines)
        .block(block("Confirm Delete", true).border_style(Style::default().fg(COL_RED)));
    f.render_widget(modal, area);
}

fn render_auth_enable_modal(f: &mut Frame, state: &AppState) {
    let area = centered_rect(55, 9, f.area());
    f.render_widget(Clear, area);

    let strategies = ["JWT Bearer Token", "Session Cookie", "API Key Header"];
    let current = strategies[state.auth_strategy_idx];

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Choose auth strategy:", Style::default().fg(COL_DIM))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Strategy: ", Style::default().fg(COL_DIM)),
            Span::styled(format!("◀ {} ▶", current), Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Auth endpoints will be auto-generated", Style::default().fg(COL_GREEN))),
        Line::from(""),
        Line::from(Span::styled("  [← →] Choose  [Enter] Enable  [Esc] Cancel", Style::default().fg(COL_DIM))),
    ];

    let modal = Paragraph::new(lines)
        .block(block("Enable Auth", true));
    f.render_widget(modal, area);
}

fn render_toast(f: &mut Frame, msg: &str) {
    let area = f.area();
    let w = (msg.len() + 6).min(60) as u16;
    let toast_area = Rect::new(
        area.x + area.width.saturating_sub(w + 2),
        area.y + 1,
        w,
        3,
    );
    f.render_widget(Clear, toast_area);
    let toast = Paragraph::new(Line::from(vec![
        Span::styled(" ✓ ", Style::default().fg(COL_GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(msg.to_string(), Style::default().fg(COL_TEXT)),
        Span::raw(" "),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(COL_GREEN)).style(Style::default().bg(Color::Rgb(20, 40, 30))));
    f.render_widget(toast, toast_area);
}

// ─── Endpoint Tester ─────────────────────────────────────────────────────────

fn render_endpoint_tester(f: &mut Frame, state: &AppState) {
    let area = f.area();

    let Some(ep) = state.project.endpoints.get(state.tester.endpoint_idx) else {
        f.render_widget(Paragraph::new("No endpoint selected.").block(block("Tester", true)), area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(area);

    // ── Left: request builder ─────────────────────────────────────────────────
    let path_count  = state.tester.path_kvs.len();
    let body_count  = state.tester.body_kvs.len();
    let has_auth    = ep.requires_auth;
    // focus layout: [0..path_count) path, [path_count..path_count+body_count) body,
    //               then auth (if needed), then SEND
    let auth_focus  = path_count + body_count;
    let send_focus  = auth_focus + if has_auth { 1 } else { 0 };

    let mut lines: Vec<Line> = Vec::new();

    // Header: method + path + hint
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(ep.method.as_str().to_string(),
            Style::default().fg(method_color(&ep.method)).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {}", ep.path),
            Style::default().fg(COL_TEXT).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            if ep.requires_auth { "  🔒 Auth required" } else { "  🔓 Public" },
            Style::default().fg(if ep.requires_auth { COL_YELLOW } else { COL_DIM }),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ─── Fill values below, Tab to move, Enter on SEND ───",
        Style::default().fg(COL_DIM),
    )));

    // Show resolved full URL
    let prefix = if let Some(model_id) = &ep.linked_model {
        state.project.models.iter()
            .find(|m| &m.id == model_id)
            .map(|m| format!("/{}", m.name.to_lowercase().replace(' ', "_")))
            .unwrap_or_default()
    } else { String::new() };
    let resolved_path: String = {
        let mut p = ep.path.clone();
        for (k, v) in &state.tester.path_kvs {
            p = p.replace(&format!("{{{}}}", k), if v.is_empty() { &k[..] } else { v });
        }
        if p == "/" { String::new() } else { p }
    };
    lines.push(Line::from(vec![
        Span::styled("  URL: ", Style::default().fg(COL_DIM)),
        Span::styled(ep.method.as_str().to_string(), Style::default().fg(method_color(&ep.method)).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" http://127.0.0.1:8000{}{}", prefix, resolved_path),
            Style::default().fg(Color::Rgb(100, 160, 255))),
    ]));
    lines.push(Line::from(""));

    // Path params section
    if !state.tester.path_kvs.is_empty() {
        lines.push(Line::from(Span::styled("  PATH PARAMS", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))));
        lines.push(Line::from(Span::styled(
            "  [p] open row picker to select from fake DB",
            Style::default().fg(COL_DIM),
        )));
        lines.push(Line::from(""));
        for (i, (k, v)) in state.tester.path_kvs.iter().enumerate() {
            let focused = state.tester.focused == i;
            render_input_row(&mut lines, k, v, focused, COL_YELLOW);
        }
        lines.push(Line::from(""));
    }

    // Body / filter fields section
    let method_upper = ep.method.as_str().to_uppercase();
    let uses_body = matches!(method_upper.as_str(), "POST" | "PUT" | "PATCH");
    let field_section_label = if uses_body {
        "  REQUEST BODY  (sent as JSON)"
    } else {
        "  FILTER PARAMS  (sent as ?key=value — leave empty to get/delete all)"
    };

    if !state.tester.body_kvs.is_empty() {
        lines.push(Line::from(Span::styled(field_section_label, Style::default().fg(COL_ACCENT).add_modifier(Modifier::BOLD))));
        lines.push(Line::from(Span::styled(
            "  [p] open row picker to auto-fill from fake DB",
            Style::default().fg(COL_DIM),
        )));
        lines.push(Line::from(""));
        for (i, (k, v)) in state.tester.body_kvs.iter().enumerate() {
            let focused = state.tester.focused == path_count + i;
            render_input_row(&mut lines, k, v, focused, COL_ACCENT);
        }
        lines.push(Line::from(""));
    } else {
        if !uses_body {
            lines.push(Line::from(Span::styled(
                "  No filter fields — this will affect ALL records.",
                Style::default().fg(COL_DIM),
            )));
            lines.push(Line::from(Span::styled(
                "  Use {id} in the path to target a specific record.",
                Style::default().fg(COL_DIM),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "  No body fields defined. Link endpoint to a model to auto-fill.",
                Style::default().fg(COL_DIM),
            )));
        }
        lines.push(Line::from(""));
    }

    // Auth token
    if has_auth {
        lines.push(Line::from(Span::styled("  AUTH TOKEN (Bearer)", Style::default().fg(COL_YELLOW).add_modifier(Modifier::BOLD))));
        lines.push(Line::from(""));
        let focused = state.tester.focused == auth_focus;
        render_input_row(&mut lines, "token", &state.tester.auth_token, focused, COL_YELLOW);
        lines.push(Line::from(""));
    }

    // SEND button
    let send_focused = state.tester.focused == send_focus;
    lines.push(if send_focused {
        Line::from(Span::styled(
            if state.tester.loading { "  [ ⏳  Sending...          ]" }
            else                    { "  [▶▶  SEND  REQUEST  ▶▶   ]" },
            Style::default().fg(COL_BG_DARK).bg(COL_GREEN).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::styled(
            "  [    SEND REQUEST   →  Enter  ]",
            Style::default().fg(COL_GREEN),
        ))
    });
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Tab/↓] next  [Shift+Tab/↑] prev  [p] picker  [Ctrl+R] send  [q] back",
        Style::default().fg(COL_DIM),
    )));

    let left_title = format!("Request Builder  ({} body field{}, {} path param{})",
        body_count, if body_count == 1 { "" } else { "s" },
        path_count, if path_count == 1 { "" } else { "s" });
    let left_para = Paragraph::new(lines)
        .block(block(&left_title, true))
        .wrap(Wrap { trim: false });
    f.render_widget(left_para, chunks[0]);

    // ── Right: response panel ─────────────────────────────────────────────────
    match &state.tester.response {
        None => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled("  Waiting for request...", Style::default().fg(COL_DIM))),
                Line::from(""),
                Line::from(Span::styled("  How to use:", Style::default().fg(COL_ACCENT))),
                Line::from(Span::styled("  1. Fill in the field values on the left", Style::default().fg(COL_TEXT))),
                Line::from(Span::styled("  2. Tab down to the SEND button", Style::default().fg(COL_TEXT))),
                Line::from(Span::styled("  3. Press Enter to fire the request", Style::default().fg(COL_TEXT))),
                Line::from(""),
                Line::from(Span::styled("  Response + status code will appear here.", Style::default().fg(COL_DIM))),
                Line::from(""),
                Line::from(Span::styled("  Server must be running first:", Style::default().fg(COL_DIM))),
                Line::from(Span::styled("  [q] → Home → [s] → [s]", Style::default().fg(COL_ACCENT))),
            ];
            f.render_widget(
                Paragraph::new(lines).block(block("Response", false)),
                chunks[1],
            );
        }
        Some(resp) => {
            let status_color = match resp.status {
                200..=299 => COL_GREEN,
                300..=399 => COL_YELLOW,
                400..=499 => COL_RED,
                500..=599 => Color::Rgb(255, 100, 0),
                _ => COL_DIM,
            };
            let status_label = match resp.status {
                200 => "200 OK", 201 => "201 Created", 204 => "204 No Content",
                400 => "400 Bad Request", 401 => "401 Unauthorized",
                403 => "403 Forbidden", 404 => "404 Not Found",
                422 => "422 Unprocessable", 500 => "500 Server Error",
                0   => "CONNECTION FAILED", _ => "Unknown",
            };

            let resp_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(4), Constraint::Min(5)])
                .split(chunks[1]);

            let status_lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Status: ", Style::default().fg(COL_DIM)),
                    Span::styled(
                        format!("  {}  {} ", resp.status, status_label),
                        Style::default().fg(COL_BG_DARK).bg(status_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("   {} ms", resp.elapsed_ms), Style::default().fg(COL_DIM)),
                ]),
            ];
            f.render_widget(
                Paragraph::new(status_lines).block(block("Response", false)),
                resp_chunks[0],
            );

            let body_lines: Vec<Line> = resp.body.lines()
                .map(|l| Line::from(Span::styled(format!("  {}", l), Style::default().fg(COL_TEXT))))
                .collect();
            f.render_widget(
                Paragraph::new(body_lines)
                    .block(block("Body  (↑↓ to scroll)", false))
                    .wrap(Wrap { trim: false }),
                resp_chunks[1],
            );
        }
    }

    // ── Row picker overlay ────────────────────────────────────────────────────
    if state.tester.picker_open {
        render_tester_picker(f, state, &ep);
    }
}

fn render_tester_picker(f: &mut Frame, state: &AppState, ep: &Endpoint) {
    let area = centered_rect(60, 16, f.area());
    f.render_widget(Clear, area);

    let rows: Vec<(u64, Vec<(String, String)>)> = ep.linked_model.as_ref()
        .and_then(|mid| state.project.get_fake_table(mid))
        .map(|t| t.rows.iter().map(|r| (r.id, r.values.clone())).collect())
        .unwrap_or_default();

    let mut lines: Vec<Line> = vec![Line::from("")];

    if rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No rows in fake DB yet. Create some via POST first.",
            Style::default().fg(COL_DIM),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Select a row — Enter to fill current field, Esc to cancel",
            Style::default().fg(COL_DIM),
        )));
        lines.push(Line::from(""));
        for (i, (row_id, values)) in rows.iter().enumerate() {
            let selected = i == state.tester.picker_row_idx;
            let row_style = if selected {
                Style::default().fg(COL_BG_DARK).bg(COL_ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(COL_TEXT)
            };
            let prefix = if selected { "  ▶ " } else { "    " };
            let vals: Vec<String> = values.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            let summary = if vals.is_empty() {
                format!("id={}", row_id)
            } else {
                format!("id={}  {}", row_id, vals.join("  "))
            };
            lines.push(Line::from(Span::styled(
                format!("{}{}", prefix, summary),
                row_style,
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [↑↓ / j k] scroll   [Enter] pick   [Esc] cancel",
        Style::default().fg(COL_DIM),
    )));

    let picker = Paragraph::new(lines)
        .block(block("Row Picker — Fake DB", true))
        .wrap(Wrap { trim: false });
    f.render_widget(picker, area);
}

/// Renders one labeled input row for the tester form.
fn render_input_row(lines: &mut Vec<Line<'static>>, label: &str, value: &str, focused: bool, accent: Color) {
    let (lbl_style, box_style) = if focused {
        (
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Rgb(220, 220, 255)).bg(Color::Rgb(30, 35, 55)),
        )
    } else {
        (
            Style::default().fg(COL_DIM),
            Style::default().fg(COL_TEXT),
        )
    };
    let cursor = if focused { "█" } else { " " };
    let prefix = if focused { "  ▶ " } else { "    " };
    // Show a "box" around the value when focused
    let display_val = if value.is_empty() && focused {
        format!("{}_", cursor)
    } else {
        format!("{}{}", value, cursor)
    };
    lines.push(Line::from(vec![
        Span::styled(prefix.to_string(), lbl_style),
        Span::styled(format!("{:<16} ", label), lbl_style),
        Span::styled("[ ", Style::default().fg(COL_BORDER)),
        Span::styled(display_val, box_style),
        Span::styled(" ]", Style::default().fg(COL_BORDER)),
    ]));
}

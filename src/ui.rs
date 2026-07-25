use crate::dictionary::{Dictionary, Language};
use crate::scrape;
use crate::storage;
use crate::render;
use anyhow::Result;
use chrono::{Duration as ChronoDuration, Local};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DateChoice {
    Today,
    PastNDays(u32), // 1..=7 days ago
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Language,
    Date,
    City,
    Running,
    Done,
}

struct AppState {
    step: Step,
    lang_idx: usize,
    lang_options: Vec<Language>,

    date_idx: usize,
    date_options: Vec<DateChoice>,

    city_names: Vec<String>, // "All" + english city names
    city_list_state: ListState,
    selected_cities: Vec<usize>, // indices into city_names (excluding "All")

    status_lines: Vec<String>,
    preview: Option<StatefulProtocol>,
}

impl AppState {
    fn new() -> Self {
        let mut city_list_state = ListState::default();
        city_list_state.select(Some(0));
        AppState {
            step: Step::Language,
            lang_idx: 0,
            lang_options: vec![Language::Kannada, Language::English],
            date_idx: 0,
            date_options: vec![
                DateChoice::Today,
                DateChoice::PastNDays(1),
                DateChoice::PastNDays(2),
                DateChoice::PastNDays(3),
                DateChoice::PastNDays(4),
                DateChoice::PastNDays(5),
                DateChoice::PastNDays(6),
                DateChoice::PastNDays(7),
            ],
            city_names: vec!["All".to_string()],
            city_list_state,
            selected_cities: Vec::new(),
            status_lines: Vec::new(),
            preview: None,
        }
    }
}

fn date_label(choice: DateChoice) -> String {
    match choice {
        DateChoice::Today => "Today".to_string(),
        DateChoice::PastNDays(n) => format!("{} day(s) ago", n),
    }
}

fn resolve_date_ddmmyyyy(choice: DateChoice) -> (String, String) {
    let target = match choice {
        DateChoice::Today => Local::now().date_naive(),
        DateChoice::PastNDays(n) => Local::now().date_naive() - ChronoDuration::days(n as i64),
    };
    (target.format("%d/%m/%Y").to_string(), target.format("%Y%m%d").to_string())
}

/// Entrypoint for the interactive UI mode.
pub async fn run_ui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let picker = Picker::from_query_stdio().ok();
    let mut app = AppState::new();
    let dict = Dictionary::load();

    let result = run_event_loop(&mut terminal, &mut app, &dict, picker).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    dict: &Dictionary,
    mut picker: Option<Picker>,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw_ui(f, app))?;

        if app.step == Step::Done {
            // Wait for a final keypress before exiting.
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    break;
                }
            }
            continue;
        }

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => move_selection(app, -1),
                    KeyCode::Down => move_selection(app, 1),
                    KeyCode::Char(' ') if app.step == Step::City => toggle_city_selection(app),
                    KeyCode::Enter => {
                        advance_step(app, dict).await?;
                        if app.step == Step::Running {
                            run_pipeline(app, dict, &mut picker).await?;
                            app.step = Step::Done;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn move_selection(app: &mut AppState, delta: i32) {
    match app.step {
        Step::Language => {
            let len = app.lang_options.len() as i32;
            app.lang_idx = (((app.lang_idx as i32 + delta) % len + len) % len) as usize;
        }
        Step::Date => {
            let len = app.date_options.len() as i32;
            app.date_idx = (((app.date_idx as i32 + delta) % len + len) % len) as usize;
        }
        Step::City => {
            let len = app.city_names.len() as i32;
            if len == 0 {
                return;
            }
            let current = app.city_list_state.selected().unwrap_or(0) as i32;
            let next = (((current + delta) % len + len) % len) as usize;
            app.city_list_state.select(Some(next));
        }
        _ => {}
    }
}

fn toggle_city_selection(app: &mut AppState) {
    if let Some(idx) = app.city_list_state.selected() {
        if idx == 0 {
            // "All" toggles/clears everything else.
            app.selected_cities.clear();
            return;
        }
        if let Some(pos) = app.selected_cities.iter().position(|&i| i == idx) {
            app.selected_cities.remove(pos);
        } else {
            app.selected_cities.push(idx);
        }
    }
}

/// Advances the wizard to the next step. When moving from Date -> City,
/// populates the city list (English names only, per spec).
async fn advance_step(app: &mut AppState, dict: &Dictionary) -> Result<()> {
    match app.step {
        Step::Language => {
            app.step = Step::Date;
        }
        Step::Date => {
            let mut names = vec!["All".to_string()];
            names.extend(dict.all_city_names(Language::English));
            app.city_names = names;
            app.city_list_state.select(Some(0));
            app.step = Step::City;
        }
        Step::City => {
            app.step = Step::Running;
        }
        _ => {}
    }
    Ok(())
}

async fn run_pipeline(
    app: &mut AppState,
    dict: &Dictionary,
    picker: &mut Option<Picker>,
) -> Result<()> {
    let lang = app.lang_options[app.lang_idx];
    let date_choice = app.date_options[app.date_idx];
    let (date_ddmmyyyy, date_ymd) = resolve_date_ddmmyyyy(date_choice);

    app.status_lines.push(format!("Scraping report for {}...", date_ddmmyyyy));

    let report = match scrape::scrape_agriculture_data(&date_ddmmyyyy, lang).await {
        Ok(r) => r,
        Err(e) => {
            app.status_lines.push(format!("Scrape failed: {}", e));
            return Ok(());
        }
    };

    let json_path = storage::write_report_json(&date_ymd, &report)?;
    app.status_lines.push(format!("Saved JSON: {}", json_path.display()));

    let cities_filter: Vec<String> = app
        .selected_cities
        .iter()
        .filter_map(|&i| app.city_names.get(i).cloned())
        .collect();
    let filter_opt = if cities_filter.is_empty() {
        None
    } else {
        Some(cities_filter.as_slice())
    };

    let outcome = render::render_report_images(&report, &date_ymd, dict, lang, filter_opt).await?;
    app.status_lines
        .push(format!("Rendered {} image(s).", outcome.written.len()));

    if outcome.written.is_empty() {
        app.status_lines.push("No images rendered - check city filter.".to_string());
        for (scraped, resolved) in outcome.skipped_cities.iter().take(10) {
            app.status_lines
                .push(format!("  skipped: '{}' -> '{}'", scraped, resolved));
        }
    }

    // Load the first rendered image as a live preview, if a terminal
    // graphics protocol (kitty/iterm2/sixel) is available.
    if let (Some(picker), Some(first)) = (picker.as_mut(), outcome.written.first()) {
        if let Ok(dyn_img) = image::open(first) {
            app.preview = Some(picker.new_resize_protocol(dyn_img));
        }
    }

    for p in &outcome.written {
        app.status_lines.push(format!("Wrote: {}", p.display()));
    }

    Ok(())
}

fn draw_ui(f: &mut Frame, app: &mut AppState) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(size);

    let title = Paragraph::new(Line::from(vec![Span::styled(
        "Raitha Darpana - Content Creator",
        Style::default().add_modifier(Modifier::BOLD).fg(Color::Green),
    )]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    match app.step {
        Step::Language => draw_language_step(f, app, chunks[1]),
        Step::Date => draw_date_step(f, app, chunks[1]),
        Step::City => draw_city_step(f, app, chunks[1]),
        Step::Running => draw_status(f, app, chunks[1], "Running..."),
        Step::Done => draw_done(f, app, chunks[1]),
    }
}

fn draw_language_step(f: &mut Frame, app: &AppState, area: Rect) {
    let items: Vec<ListItem> = app
        .lang_options
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let marker = if i == app.lang_idx { "> " } else { "  " };
            ListItem::new(format!("{}{}", marker, l.as_str()))
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Select Language (Up/Down, Enter)"),
    );
    f.render_widget(list, area);
}

fn draw_date_step(f: &mut Frame, app: &AppState, area: Rect) {
    let items: Vec<ListItem> = app
        .date_options
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let marker = if i == app.date_idx { "> " } else { "  " };
            ListItem::new(format!("{}{}", marker, date_label(*d)))
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Select Date (Up/Down, Enter)"),
    );
    f.render_widget(list, area);
}

fn draw_city_step(f: &mut Frame, app: &mut AppState, area: Rect) {
    let selected = app.selected_cities.clone();
    let items: Vec<ListItem> = app
        .city_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let checked = if i == 0 {
                selected.is_empty()
            } else {
                selected.contains(&i)
            };
            let box_char = if checked { "[x]" } else { "[ ]" };
            ListItem::new(format!("{} {}", box_char, name))
        })
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select City (Up/Down, Space to toggle, Enter to confirm)"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_stateful_widget(list, area, &mut app.city_list_state);
}

fn draw_status(f: &mut Frame, app: &AppState, area: Rect, header: &str) {
    let mut lines: Vec<Line> = vec![Line::from(header.to_string())];
    lines.extend(app.status_lines.iter().map(|s| Line::from(s.clone())));
    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(p, area);
}

fn draw_done(f: &mut Frame, app: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_status(f, app, chunks[0], "Done. Press any key to exit.");

    if let Some(proto) = app.preview.as_mut() {
        let image_widget = StatefulImage::default();
        f.render_stateful_widget(image_widget, chunks[1], proto);
    } else {
        let p = Paragraph::new("No preview available (terminal graphics protocol not detected).")
            .block(Block::default().borders(Borders::ALL).title("Preview"));
        f.render_widget(p, chunks[1]);
    }
}
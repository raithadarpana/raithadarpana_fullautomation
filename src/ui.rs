use crate::data::AgriculturalReport;
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
    FetchingCities,
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

    // Populated once the data fetch (post-date-selection) completes, and
    // reused for rendering so we never scrape the same date/language
    // twice in one wizard run.
    report: Option<AgriculturalReport>,
    date_ddmmyyyy: String,
    date_ymd: String,
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
            report: None,
            date_ddmmyyyy: String::new(),
            date_ymd: String::new(),
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

    let mut app = AppState::new();
    let dict = Dictionary::load();

    let result = run_event_loop(&mut terminal, &mut app, &dict).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    dict: &Dictionary,
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
                    KeyCode::Enter => match app.step {
                        Step::Language => {
                            app.step = Step::Date;
                        }
                        Step::Date => {
                            app.step = Step::FetchingCities;
                            terminal.draw(|f| draw_ui(f, app))?;
                            fetch_cities(terminal, app, dict).await?;
                        }
                        Step::City => {
                            app.step = Step::Running;
                            terminal.draw(|f| draw_ui(f, app))?;
                            run_pipeline(terminal, app, dict).await?;
                            app.step = Step::Done;
                        }
                        _ => {}
                    },
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

/// Pushes a status line and immediately redraws the terminal so the
/// person sees progress as each stage happens, rather than a frozen
/// screen while a long-running scrape/render blocks the event loop.
fn push_status(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    msg: String,
) -> Result<()> {
    log::info!("{}", msg);
    app.status_lines.push(msg);
    terminal.draw(|f| draw_ui(f, app))?;
    Ok(())
}

/// Runs after the date is selected: fetches and scrapes the report for
/// that date/language, then narrows the city list down to only the
/// cities actually present in the fetched data (rather than every city
/// the dictionary knows about) before handing off to the City step. The
/// fetched report is cached on `AppState` so `run_pipeline` doesn't need
/// to scrape a second time.
async fn fetch_cities(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    dict: &Dictionary,
) -> Result<()> {
    let lang = app.lang_options[app.lang_idx];
    let date_choice = app.date_options[app.date_idx];
    let (date_ddmmyyyy, date_ymd) = resolve_date_ddmmyyyy(date_choice);
    app.date_ddmmyyyy = date_ddmmyyyy.clone();
    app.date_ymd = date_ymd;

    push_status(
        terminal,
        app,
        format!("Fetching market report for {}...", date_ddmmyyyy),
    )?;

    match scrape::scrape_agriculture_data(&date_ddmmyyyy, lang).await {
        Ok(report) => {
            push_status(
                terminal,
                app,
                format!(
                    "Fetched data for {} city/cities. Preparing selection...",
                    report.cities.len()
                ),
            )?;

            // Only cities actually present in the fetched report are
            // offered, deduplicated and resolved to their canonical
            // English name (used downstream as the render filter).
            let mut names = vec!["All".to_string()];
            let mut seen = std::collections::HashSet::new();
            for city in &report.cities {
                let english = storage::resolve_english_city_name(dict, &city.city_name);
                if seen.insert(english.clone()) {
                    names.push(english);
                }
            }

            app.city_names = names;
            app.city_list_state.select(Some(0));
            app.selected_cities.clear();
            app.report = Some(report);
            app.step = Step::City;
        }
        Err(e) => {
            push_status(terminal, app, format!("Fetch failed: {}", e))?;
            app.step = Step::Date;
        }
    }

    Ok(())
}

async fn run_pipeline(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    dict: &Dictionary,
) -> Result<()> {
    let lang = app.lang_options[app.lang_idx];
    let date_ymd = app.date_ymd.clone();

    let report = match app.report.clone() {
        Some(r) => r,
        None => {
            push_status(terminal, app, "No data was fetched; aborting.".to_string())?;
            return Ok(());
        }
    };

    let json_path = storage::write_report_json(&date_ymd, &report)?;
    push_status(terminal, app, format!("Saved JSON: {}", json_path.display()))?;

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

    // A progress closure that pushes a status line and redraws the
    // terminal for every stage of the render (browser launch, then each
    // city/variant) so the person sees live feedback instead of a
    // frozen screen while headless_chrome works.
    let progress = |msg: &str| {
        app.status_lines.push(msg.to_string());
        let _ = terminal.draw(|f| draw_ui(f, app));
    };

    let outcome =
        render::render_report_images(&report, &date_ymd, dict, lang, filter_opt, progress, true).await?;

    push_status(
        terminal,
        app,
        format!("Rendered {} image(s).", outcome.written.len()),
    )?;

    if outcome.written.is_empty() {
        push_status(
            terminal,
            app,
            "No images rendered - check city filter.".to_string(),
        )?;
        for (scraped, resolved) in outcome.skipped_cities.iter().take(10) {
            push_status(
                terminal,
                app,
                format!("  skipped: '{}' -> '{}'", scraped, resolved),
            )?;
        }
    }

    for p in &outcome.written {
        push_status(terminal, app, format!("Wrote: {}", p.display()))?;
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
        Step::FetchingCities => draw_status(f, app, chunks[1], "Fetching data..."),
        Step::City => draw_city_step(f, app, chunks[1]),
        Step::Running => draw_status(f, app, chunks[1], "Rendering..."),
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
    draw_status(f, app, area, "Done. Press any key to exit.");
}
use crate::data::{parse_agricultural_report, AgriculturalReport};
use crate::dictionary::Language;
use anyhow::Result;
use headless_chrome::protocol::cdp::types::Event;
use headless_chrome::{Browser, Tab};
use std::sync::{Arc, Mutex};

/// Scrapes the agricultural market report for a given date (dd/mm/yyyy)
/// and language from the Karnataka government site.
pub async fn scrape_agriculture_data(date_ddmmyyyy: &str, lang: Language) -> Result<AgriculturalReport> {
    let base_url = match lang {
        Language::Kannada => "https://krama.karnataka.gov.in/Kannada",
        Language::English => "https://krama.karnataka.gov.in/Reports",
    };

    let browser = Browser::default()?;
    let tab = browser.new_tab()?;

    // The site is an ASP.NET WebForms app that validates the submitted
    // date with a plain JS `alert(...)` (e.g. "no data available",
    // "invalid date", weekends/holidays) instead of showing inline
    // errors. Headless Chrome has no UI to dismiss that alert, so the
    // page just hangs forever waiting for it -- which surfaced as a
    // generic "Timeout waiting for URL containing: MaraketsRep", with no
    // hint that a dialog was actually the cause. This listener captures
    // the dialog's message and auto-accepts it (as if a user clicked
    // "OK"), so the page can proceed (or at least tell us why it didn't).
    let dialog_message: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    {
        let tab_for_dialog = tab.clone();
        let dialog_message = dialog_message.clone();
        tab.add_event_listener(Arc::new(move |event: &Event| {
            if let Event::PageJavascriptDialogOpening(ev) = event {
                *dialog_message.lock().unwrap() = Some(ev.params.message.clone());
                log::warn!("Site raised a JS dialog: {}", ev.params.message);
                let dialog = tab_for_dialog.get_dialog();
                let _ = dialog.accept(None);
            }
        }))?;
    }

    let navigate_url = format!("{}/Main_rep", base_url);
    tab.navigate_to(&navigate_url)?;
    // Confirm the form has actually loaded rather than guessing with a
    // fixed sleep -- a slow response (or the site being down) used to
    // silently proceed into the `evaluate` calls below with elements
    // that don't exist yet.
    tab.wait_for_element("#_ctl0_MainContent_TxtDate")
        .map_err(|e| anyhow::anyhow!("Report form never loaded at {}: {}", navigate_url, e))?;

    tab.evaluate(
        &format!(
            r#"document.getElementById('_ctl0_MainContent_TxtDate').value = '{}'"#,
            date_ddmmyyyy
        ),
        false,
    )?;

    tab.evaluate(
        r#"document.getElementById('_ctl0_MainContent_RadBtnSel_1').click()"#,
        false,
    )?;

    tab.evaluate(
        r#"document.getElementById('_ctl0_MainContent_BtnRep').click()"#,
        false,
    )?;

    wait_for_url(&tab, "MaraketsRep", &dialog_message, date_ddmmyyyy)?;

    tab.evaluate(
        r#"document.querySelector('[id^="_ctl0_MainContent_ChkAll"]').checked = true"#,
        false,
    )?;

    tab.evaluate(
        r#"document.getElementById('_ctl0_MainContent_BtnRep').click()"#,
        false,
    )?;

    wait_for_url(&tab, "DailyMar", &dialog_message, date_ddmmyyyy)?;
    std::thread::sleep(std::time::Duration::from_secs(5));

    let table_html = tab.evaluate(
        r#"
        var table = document.getElementById('printtable') || document.querySelector('#divprint table');
        table ? table.outerHTML : '<p>No table found</p>';
        "#,
        false,
    )?;

    let html_string = table_html.value.unwrap().as_str().unwrap().to_string();
    let report = parse_agricultural_report(&html_string)?;

    log::info!("Report Date: {}", report.report_date);
    log::info!("Total Cities: {}", report.cities.len());
    for city in &report.cities {
        log::info!("City: {} - Commodities: {}", city.city_name, city.commodities.len());
    }

    Ok(report)
}

fn wait_for_url(tab: &Tab, target: &str, dialog_message: &Arc<Mutex<Option<String>>>, date_ddmmyyyy: &str) -> Result<()> {
    for _ in 0..30 {
        let url = tab.get_url();
        if url.contains(target) {
            return Ok(());
        }
        // Surface a site-raised dialog immediately rather than waiting
        // out the full timeout first -- it's already been auto-accepted
        // by the listener above, so waiting further would just time out
        // on a page that's never going to navigate.
        if let Some(msg) = dialog_message.lock().unwrap().clone() {
            anyhow::bail!(
                "Site rejected date {}: \"{}\" (a JS dialog fired instead of navigating to a page containing '{}'). This usually means no market report exists for that date (weekend/holiday) or the date format the site expects has changed.",
                date_ddmmyyyy,
                msg,
                target
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    anyhow::bail!(
        "Timeout waiting for URL containing: {} (date: {}, ended up at: {})",
        target,
        date_ddmmyyyy,
        tab.get_url()
    )
}
use crate::data::{parse_agricultural_report, AgriculturalReport};
use crate::dictionary::Language;
use anyhow::Result;
use headless_chrome::{Browser, Tab};

/// Scrapes the agricultural market report for a given date (dd/mm/yyyy)
/// and language from the Karnataka government site.
pub async fn scrape_agriculture_data(date_ddmmyyyy: &str, lang: Language) -> Result<AgriculturalReport> {
    let base_url = match lang {
        Language::Kannada => "https://krama.karnataka.gov.in/Kannada",
        Language::English => "https://krama.karnataka.gov.in/Reports",
    };

    let browser = Browser::default()?;
    let tab = browser.new_tab()?;

    let navigate_url = format!("{}/Main_rep", base_url);
    tab.navigate_to(&navigate_url)?;
    std::thread::sleep(std::time::Duration::from_secs(2));

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

    wait_for_url(&tab, "MaraketsRep")?;

    tab.evaluate(
        r#"document.querySelector('[id^="_ctl0_MainContent_ChkAll"]').checked = true"#,
        false,
    )?;

    tab.evaluate(
        r#"document.getElementById('_ctl0_MainContent_BtnRep').click()"#,
        false,
    )?;

    wait_for_url(&tab, "DailyMar")?;
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

fn wait_for_url(tab: &Tab, target: &str) -> Result<()> {
    for _ in 0..30 {
        let url = tab.get_url();
        if url.contains(target) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    anyhow::bail!("Timeout waiting for URL containing: {}", target)
}

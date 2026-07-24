pub mod data;
pub mod render;

use anyhow::Result;
use clap::Parser;
use chrono::Local;
use headless_chrome::Browser;
use std::fs;

use data::{AgriculturalReport, parse_agricultural_report};
use render::{render_html_to_image};

#[derive(Parser, Debug)]
#[command(name = "Raitha Darpana Content Creator")]
#[command(about = "Scrapes market price reports from Karnataka gov site and creates city wise cover images", long_about = None)]
struct Args {
    /// Language to use (kannada or english)
    #[arg(short, long, default_value = "kannada")]
    language: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let args = Args::parse();
    let today = Local::now().format("%d/%m/%Y").to_string();
    
    let report = scrape_agriculture_data(&today, &args.language).await?;
    let json = serde_json::to_string_pretty(&report)?;
    fs::write("report.json", &json)?;

    print!("Extracted data:\n{}", json);
    // Fill HTML template with table data and render to image
    render_html_to_image(&report).await?;
    
    Ok(())
}

async fn scrape_agriculture_data(date: &str, language: &str) -> Result<AgriculturalReport> {
    // Determine base URL based on language
    let base_url = match language.to_lowercase().as_str() {
        "kannada" => "https://krama.karnataka.gov.in/Kannada",
        "english" => "https://krama.karnataka.gov.in/Reports",
        other => anyhow::bail!("Unsupported language: {}. Use 'kannada' or 'english'", other),
    };

    // Launch Chrome
    let browser = Browser::default()?;
    let tab = browser.new_tab()?;
    
     // Step 1: Navigate to reports page with dynamic language
    let navigate_url = format!("{}/Main_rep", base_url);
    tab.navigate_to(&navigate_url)?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Step 2: Fill date input
    tab.evaluate(
        &format!(
            r#"document.getElementById('_ctl0_MainContent_TxtDate').value = '{}'"#,
            date
        ),
        false,
    )?;
    
    // Step 2: Select radio button
    tab.evaluate(
        r#"document.getElementById('_ctl0_MainContent_RadBtnSel_1').click()"#,
        false,
    )?;
    
    // Step 3: Click submit button
    tab.evaluate(
        r#"document.getElementById('_ctl0_MainContent_BtnRep').click()"#,
        false,
    )?;
    
    // Step 4: Wait for page load to MaraketsRep
    wait_for_url(&tab, "MaraketsRep")?;
    
    // Step 5: Select checkbox and click button
    tab.evaluate(
    r#"document.querySelector('[id^="_ctl0_MainContent_ChkAll"]').checked = true"#,
    false,
    )?;

    
    tab.evaluate(
        r#"document.getElementById('_ctl0_MainContent_BtnRep').click()"#,
        false,
    )?;
    
    // Step 6: Wait for page load to DailyMar
    wait_for_url(&tab, "DailyMar")?;
    std::thread::sleep(std::time::Duration::from_secs(5));
    
    // Step 7: Extract table data - fallback to div > table if printtable id doesn't exist
    let table_html = tab.evaluate(
        r#"
        var table = document.getElementById('printtable') || document.querySelector('#divprint table');
        table ? table.outerHTML : '<p>No table found</p>';
        "#,
        false,
    )?;

    let html_string = table_html.value.unwrap().as_str().unwrap().to_string();

    // Parse HTML into structured data
    let report = parse_agricultural_report(&html_string)?;
    
    log::info!("Report Date: {}", report.report_date);
    log::info!("Total Cities: {}", report.cities.len());
    
    for city in &report.cities {
        log::info!("City: {} - Commodities: {}", city.city_name, city.commodities.len());
    }
    
    Ok(report)
    
}


fn wait_for_url(tab: &headless_chrome::Tab, target: &str) -> Result<()> {
    for _ in 0..30 {
        let url = tab.get_url();
        if url.contains(target) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    anyhow::bail!("Timeout waiting for URL containing: {}", target)
}
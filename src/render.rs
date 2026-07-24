use crate::data::AgriculturalReport;
use anyhow::Result;
use base64::Engine;
use chrono::Local;
use headless_chrome::{
    Browser,
    LaunchOptions,
    protocol::cdp::Page::{self, CaptureScreenshotFormatOption},
};
use std::fs;

pub async fn render_html_to_image(report: &AgriculturalReport) -> Result<()> {
    // Build HTML table rows dynamically from parsed data
    let mut table_rows = String::new();
    
    for city in &report.cities {
        table_rows.push_str(&format!(
            "<tr><td colspan='8' style='font-weight: bold; background-color: #f0f0f0; padding: 10px;'>{}</td></tr>",
            escape_html(&city.city_name)
        ));
        
        for commodity in &city.commodities {
            table_rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&commodity.commodity),
                escape_html(&commodity.variety),
                escape_html(&commodity.grade),
                commodity.arrivals,
                escape_html(&commodity.units),
                commodity.min_rs,
                commodity.max_rs,
                commodity.modal_rs,
            ));
        }
    }
    
    let html_template = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        h1 {{ color: #006600; }}
        .date {{ color: #666; margin-bottom: 20px; }}
        table {{ border-collapse: collapse; width: 100%; margin-top: 20px; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #4CAF50; color: white; font-weight: bold; }}
        td {{ font-size: 12pt; }}
    </style>
</head>
<body>
    <h1>ಉತ್ಪನ್ನವಾರು ದೈನಂದಿನ ವರದಿ (Daily Agricultural Report)</h1>
    <div class="date">Report Date: {}</div>
    
    <table border="1">
        <thead>
            <tr>
                <th>Commodity</th>
                <th>Variety</th>
                <th>Grade</th>
                <th>Arrivals</th>
                <th>Units</th>
                <th>Min Price</th>
                <th>Max Price</th>
                <th>Modal Price</th>
            </tr>
        </thead>
        <tbody>
            {}
        </tbody>
    </table>
</body>
</html>
"#,
        report.report_date,
        table_rows
    );
    
    fs::write("report.html", &html_template)?;
    
    // Use headless-chrome to render and take screenshot
    let browser = Browser::new(LaunchOptions::default_builder().build().unwrap())?;
    let tab = browser.new_tab()?;
    
    let url = format!("file://{}/report.html", std::env::current_dir()?.display());
    tab.navigate_to(&url)?;

    // The generated report is a local document, so do not wait for selectors
    // from the source site. Waiting for the report table also ensures that the
    // document has been parsed before its bounds are requested.
    let viewport = tab
        .wait_for_element("table")?
        .get_box_model()?
        .margin_viewport();
    
    // `Tab::capture_screenshot` in headless-chrome 1.0.22 does not expose
    // Chrome's `captureBeyondViewport` option, so it can return a viewport
    // image with blank space for content below the fold. Call the CDP method
    // directly and explicitly capture the complete clipped region.
    let screenshot = tab.call_method(Page::CaptureScreenshot {
        format: Some(CaptureScreenshotFormatOption::Png),
        quality: None,
        clip: Some(viewport),
        from_surface: Some(true),
        capture_beyond_viewport: Some(true),
        optimize_for_speed: None,
    })?;
    let png_data = base64::engine::general_purpose::STANDARD.decode(screenshot.data)?;
    
    let filename = format!("agriculture_report_{}.png", Local::now().format("%Y%m%d_%H%M%S"));
    fs::write(&filename, png_data)?;
    
    println!("Report saved to: {}", filename);
    
    Ok(())
}

// Helper function to escape HTML special characters
fn escape_html(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#39;")
}

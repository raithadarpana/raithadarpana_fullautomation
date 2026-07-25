use crate::data::CityMarketData;
use crate::dictionary::{Dictionary, Language};

/// Instagram cover: 4:5 portrait (e.g. 1080x1350).
pub const INSTAGRAM_WIDTH: u32 = 1080;
pub const INSTAGRAM_HEIGHT: u32 = 1350;

/// YouTube cover: 16:9 landscape (e.g. 1280x720).
pub const YOUTUBE_WIDTH: u32 = 1280;
pub const YOUTUBE_HEIGHT: u32 = 720;

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn commodity_rows(city: &CityMarketData, dict: &Dictionary, lang: Language) -> String {
    let mut rows = String::new();
    for c in &city.commodities {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&dict.commodity_display(&c.commodity, lang)),
            escape_html(&dict.variety_display(&c.variety, lang)),
            c.min_rs,
            c.max_rs,
            c.modal_rs,
        ));
    }
    rows
}

fn table_headers(dict: &Dictionary, lang: Language) -> String {
    format!(
        "<tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>",
        escape_html(&dict.term("commodity", lang)),
        escape_html(&dict.term("variety", lang)),
        escape_html(&dict.term("min_price", lang)),
        escape_html(&dict.term("max_price", lang)),
        escape_html(&dict.term("modal_price", lang)),
    )
}

/// Builds a basic Instagram (4:5) HTML cover for one city.
pub fn instagram_html(
    city: &CityMarketData,
    report_date: &str,
    dict: &Dictionary,
    lang: Language,
) -> String {
    let city_name = dict.city_display(&city.city_name, lang);
    let title = dict.term("title", lang);
    let report_date_label = dict.term("report_date", lang);
    let quintal_label = dict.term("quintal", lang);
    let subscribe_label = dict.term("subscribe", lang);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<style>
  body {{ margin:0; width:{width}px; height:{height}px; font-family: 'Noto Sans Kannada', Arial, sans-serif;
          background: linear-gradient(180deg, #e8f5e9 0%, #ffffff 60%); }}
  .container {{ padding: 40px; box-sizing: border-box; height: 100%; display:flex; flex-direction:column; }}
  h1 {{ color:#1b5e20; font-size: 40px; margin: 0 0 6px 0; }}
  h2 {{ color:#2e7d32; font-size: 32px; margin: 0 0 4px 0; }}
  .date {{ color:#555; font-size: 22px; margin-bottom: 20px; }}
  table {{ width:100%; border-collapse: collapse; font-size: 22px; }}
  th, td {{ border: 1px solid #c8e6c9; padding: 10px; text-align: left; }}
  th {{ background:#4CAF50; color:#fff; }}
  .unit-note {{ margin-top:10px; font-size:18px; color:#555; }}
  .footer {{ margin-top:auto; text-align:center; font-size:24px; color:#1b5e20; font-weight:bold; }}
</style>
</head>
<body>
  <div class="container">
    <h1>{title}</h1>
    <h2>{city_name}</h2>
    <div class="date">{report_date_label}: {report_date}</div>
    <table>
      <thead>{headers}</thead>
      <tbody>{rows}</tbody>
    </table>
    <div class="unit-note">({quintal_label})</div>
    <div class="footer">{subscribe_label}</div>
  </div>
</body>
</html>"#,
        width = INSTAGRAM_WIDTH,
        height = INSTAGRAM_HEIGHT,
        title = escape_html(&title),
        city_name = escape_html(&city_name),
        report_date_label = escape_html(&report_date_label),
        report_date = escape_html(report_date),
        headers = table_headers(dict, lang),
        rows = commodity_rows(city, dict, lang),
        quintal_label = escape_html(&quintal_label),
        subscribe_label = escape_html(&subscribe_label),
    )
}

/// Builds a basic YouTube (16:9) HTML cover for one city.
pub fn youtube_html(
    city: &CityMarketData,
    report_date: &str,
    dict: &Dictionary,
    lang: Language,
) -> String {
    let city_name = dict.city_display(&city.city_name, lang);
    let title = dict.term("title", lang);
    let report_date_label = dict.term("report_date", lang);
    let quintal_label = dict.term("quintal", lang);
    let subscribe_label = dict.term("subscribe", lang);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<style>
  body {{ margin:0; width:{width}px; height:{height}px; font-family: 'Noto Sans Kannada', Arial, sans-serif;
          background: linear-gradient(135deg, #e8f5e9 0%, #ffffff 70%); }}
  .container {{ padding: 30px 50px; box-sizing: border-box; height: 100%; display:flex; flex-direction:column; }}
  .top {{ display:flex; justify-content:space-between; align-items:baseline; }}
  h1 {{ color:#1b5e20; font-size: 34px; margin: 0; }}
  h2 {{ color:#2e7d32; font-size: 30px; margin: 0; }}
  .date {{ color:#555; font-size: 18px; margin: 6px 0 14px 0; }}
  table {{ width:100%; border-collapse: collapse; font-size: 18px; }}
  th, td {{ border: 1px solid #c8e6c9; padding: 8px 10px; text-align: left; }}
  th {{ background:#4CAF50; color:#fff; }}
  .unit-note {{ margin-top:8px; font-size:15px; color:#555; }}
  .footer {{ margin-top:auto; text-align:right; font-size:20px; color:#1b5e20; font-weight:bold; }}
</style>
</head>
<body>
  <div class="container">
    <div class="top">
      <h1>{title}</h1>
      <h2>{city_name}</h2>
    </div>
    <div class="date">{report_date_label}: {report_date}</div>
    <table>
      <thead>{headers}</thead>
      <tbody>{rows}</tbody>
    </table>
    <div class="unit-note">({quintal_label})</div>
    <div class="footer">{subscribe_label}</div>
  </div>
</body>
</html>"#,
        width = YOUTUBE_WIDTH,
        height = YOUTUBE_HEIGHT,
        title = escape_html(&title),
        city_name = escape_html(&city_name),
        report_date_label = escape_html(&report_date_label),
        report_date = escape_html(report_date),
        headers = table_headers(dict, lang),
        rows = commodity_rows(city, dict, lang),
        quintal_label = escape_html(&quintal_label),
        subscribe_label = escape_html(&subscribe_label),
    )
}
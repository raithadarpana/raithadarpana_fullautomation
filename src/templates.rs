use crate::assets::BrandingAssets;
use crate::data::{CityMarketData, CommodityEntry};
use crate::dictionary::{Dictionary, Language};

/// Instagram cover: 4:5 portrait (e.g. 1080x1350).
pub const INSTAGRAM_WIDTH: u32 = 1080;
pub const INSTAGRAM_HEIGHT: u32 = 1350;

/// YouTube cover: 16:9 landscape (e.g. 1280x720).
pub const YOUTUBE_WIDTH: u32 = 1280;
pub const YOUTUBE_HEIGHT: u32 = 720;

/// Max number of commodities shown per city cover, sorted by arrivals
/// quantity (descending) before capping so the highest-volume commodities
/// are the ones that make the cut.
const MAX_COMMODITIES_SHOWN: usize = 5;

/// Update these to match your actual channel handles/links.
const INSTAGRAM_HANDLE: &str = "@raitha_darpana";
const YOUTUBE_CHANNEL_URL: &str = "https://youtube.com/@raitha_darpana";

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Returns this city's commodities sorted by arrivals quantity
/// (descending), capped at `MAX_COMMODITIES_SHOWN` entries.
fn top_commodities(city: &CityMarketData) -> Vec<&CommodityEntry> {
    let mut items: Vec<&CommodityEntry> = city.commodities.iter().collect();
    items.sort_by(|a, b| {
        b.arrivals
            .partial_cmp(&a.arrivals)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items.truncate(MAX_COMMODITIES_SHOWN);
    items
}

fn commodity_rows(city: &CityMarketData, dict: &Dictionary, lang: Language) -> String {
    let mut rows = String::new();
    for c in top_commodities(city) {
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

/// Instagram-specific row/header builders: mirrors the columns in
/// `templates/ig_template.html` (commodity, variety, grade, arrivals, min,
/// max -- no modal price), using the same sorted+capped commodity list.
fn ig_table_headers(dict: &Dictionary, lang: Language) -> String {
    format!(
        "<tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>",
        escape_html(&dict.term("commodity", lang)),
        escape_html(&dict.term("variety", lang)),
        escape_html(&dict.term("grade", lang)),
        escape_html(&dict.term("arrivals", lang)),
        escape_html(&dict.term("min_price", lang)),
        escape_html(&dict.term("max_price", lang)),
    )
}

fn ig_commodity_rows(city: &CityMarketData, dict: &Dictionary, lang: Language) -> String {
    let mut rows = String::new();
    for c in top_commodities(city) {
        let unit_label = dict.unit_display(&c.units, lang);
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{} {}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&dict.commodity_display(&c.commodity, lang)),
            escape_html(&dict.variety_display(&c.variety, lang)),
            escape_html(&dict.grade_display(&c.grade, lang)),
            format_arrivals(c.arrivals),
            escape_html(&unit_label),
            c.min_rs,
            c.max_rs,
        ));
    }
    rows
}

/// `Language::as_str` returns full words ("kannada"/"english") for CLI
/// parsing; the HTML `lang` attribute wants short codes instead.
fn html_lang_code(lang: Language) -> &'static str {
    match lang {
        Language::Kannada => "kn",
        Language::English => "en",
    }
}

fn format_arrivals(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{:.2}", value)
    }
}

/// Builds the branded Instagram (4:5) HTML cover for one city, based on
/// `templates/ig_template.html`. Logo and background come from
/// `BrandingAssets` (loaded at runtime from files next to the binary --
/// see `assets.rs`); either can be missing and the layout still renders
/// sensibly (logo falls back to channel-name text, background falls back
/// to the plain dark-green gradient).
pub fn instagram_html(
    city: &CityMarketData,
    report_date: &str,
    dict: &Dictionary,
    lang: Language,
    assets: &BrandingAssets,
) -> String {
    let city_name = dict.city_display(&city.city_name, lang);
    let title = dict.term("title", lang);
    let report_date_label = dict.term("report_date", lang);
    let subscribe_label = dict.term("subscribe", lang);
    let watch_youtube_label = dict.term("watch_on_youtube", lang);

    let logo_style = match &assets.logo_data_uri {
        Some(uri) => format!(
            "background-image:url('{}');background-size:cover;background-position:center;",
            uri
        ),
        None => String::new(),
    };
    let logo_fallback_text = if assets.logo_data_uri.is_some() {
        String::new()
    } else {
        escape_html(&title)
    };

    let background_style = match &assets.background_data_uri {
        Some(uri) => format!(
            "background-image:linear-gradient(rgba(6,59,6,.55),rgba(5,40,5,.75)),url('{}');background-size:cover;background-position:center;",
            uri
        ),
        None => String::new(),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="{lang_code}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>
*{{
    margin:0;
    padding:0;
    box-sizing:border-box;
    font-family:'Noto Sans Kannada', Arial, sans-serif;
}}

body{{
    width:{width}px;
    height:{height}px;
    background:#0b2f0b;
    overflow:hidden;
}}

.container{{
    width:100%;
    height:100%;
    background:linear-gradient(#063b06,#052805);
    {background_style}
    color:#fff;
    display:flex;
    flex-direction:column;
}}

.header{{
    display:flex;
    align-items:center;
    gap:25px;
    padding:30px;
}}

.logo{{
    width:150px;
    height:150px;
    flex-shrink:0;
    background:#fff;
    border-radius:50%;
    border:6px solid #2d7c2d;
    display:flex;
    justify-content:center;
    align-items:center;
    color:#0d5a0d;
    font-size:22px;
    font-weight:bold;
    text-align:center;
    padding:10px;
    {logo_style}
}}

.title{{
    flex:1;
}}

.title h1{{
    font-size:44px;
    line-height:1.15;
    color:#fff;
}}

.info-bar{{
    display:flex;
    justify-content:space-between;
    margin:0 30px 20px;
    gap:20px;
}}

.date-label{{
    flex:1;
    background:rgba(33,95,23,.85);
    color:#fff;
    padding:16px;
    border-radius:12px;
    font-size:24px;
    font-weight:bold;
}}

.date-box{{
    background:#fff;
    color:#111;
    font-size:28px;
    font-weight:bold;
    display:flex;
    justify-content:center;
    align-items:center;
    padding:0 24px;
    border-radius:12px;
}}

.market{{
    width:90%;
    margin:0 auto 15px;
    background:#d71e1e;
    color:#fff;
    text-align:center;
    padding:15px;
    border-radius:15px;
    font-size:30px;
    font-weight:bold;
}}

table{{
    width:92%;
    margin:0 auto 20px;
    border-collapse:collapse;
    background:#fff;
    color:#222;
}}

th{{
    background:#1c6d17;
    color:#fff;
    padding:12px;
    border:1px solid #ddd;
    font-size:17px;
}}

td{{
    padding:10px;
    border:1px solid #ddd;
    text-align:center;
    font-size:16px;
}}

tbody tr:nth-child(even){{
    background:#f3fff0;
}}

.footer{{
    margin-top:auto;
    display:flex;
    flex-direction:column;
    align-items:center;
    gap:10px;
    padding:20px 30px 30px;
}}

.socials{{
    display:flex;
    align-items:center;
    gap:16px;
    flex-wrap:wrap;
    justify-content:center;
}}

.subscribe{{
    background:#e61d1d;
    color:#fff;
    padding:12px 32px;
    border-radius:50px;
    font-size:22px;
    font-weight:bold;
}}

.instagram-handle{{
    background:linear-gradient(45deg,#f58529,#dd2a7b,#8134af,#515bd4);
    color:#fff;
    padding:12px 24px;
    border-radius:50px;
    font-size:20px;
    font-weight:bold;
}}

.youtube-link{{
    color:#fff;
    font-size:16px;
    opacity:.9;
    word-break:break-all;
    text-align:center;
}}
</style>
</head>
<body>

<div class="container">

  <div class="header">
    <div class="logo">{logo_fallback_text}</div>
    <div class="title">
      <h1>{title}</h1>
    </div>
  </div>

  <div class="info-bar">
    <div class="date-label">{report_date_label}</div>
    <div class="date-box">{report_date}</div>
  </div>

  <div class="market">{city_name}</div>

  <table>
    <thead>{headers}</thead>
    <tbody>{rows}</tbody>
  </table>

  <div class="footer">
    <div class="socials">
      <div class="subscribe">{subscribe_label}</div>
      <div class="instagram-handle">{instagram_handle}</div>
    </div>
    <div class="youtube-link">{watch_youtube_label}: {youtube_url}</div>
  </div>

</div>

</body>
</html>"#,
        lang_code = html_lang_code(lang),
        width = INSTAGRAM_WIDTH,
        height = INSTAGRAM_HEIGHT,
        background_style = background_style,
        logo_style = logo_style,
        logo_fallback_text = logo_fallback_text,
        title = escape_html(&title),
        report_date_label = escape_html(&report_date_label),
        report_date = escape_html(report_date),
        city_name = escape_html(&city_name),
        headers = ig_table_headers(dict, lang),
        rows = ig_commodity_rows(city, dict, lang),
        subscribe_label = escape_html(&subscribe_label),
        instagram_handle = escape_html(INSTAGRAM_HANDLE),
        watch_youtube_label = escape_html(&watch_youtube_label),
        youtube_url = escape_html(YOUTUBE_CHANNEL_URL),
    )
}

/// Builds a basic YouTube (16:9) HTML cover for one city. Branding
/// (logo/background) will be layered in when the YouTube template is
/// designed; for now this keeps the existing simple layout but applies
/// the same sort-by-arrivals + cap-at-5 rule as the Instagram cover.
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
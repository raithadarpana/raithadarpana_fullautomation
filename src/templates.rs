use crate::assets::BrandingAssets;
use crate::data::{CityMarketData, CommodityEntry};
use crate::dictionary::{Dictionary, Language};

/// Instagram cover: portrait, 941x1672.
pub const INSTAGRAM_WIDTH: u32 = 941;
pub const INSTAGRAM_HEIGHT: u32 = 1672;

/// YouTube cover: 16:9 landscape (e.g. 1280x720).
pub const YOUTUBE_WIDTH: u32 = 1280;
pub const YOUTUBE_HEIGHT: u32 = 720;

/// Max number of commodities shown per city cover, sorted by arrivals
/// quantity (descending) before capping so the highest-volume commodities
/// are the ones that make the cut.
const MAX_COMMODITIES_SHOWN: usize = 10;

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
///
/// `pub(crate)` so `voiceover.rs` can use the exact same
/// filtered+ordered list when generating narration scripts, keeping the
/// voiceover in sync with what's shown in the rendered cover images.
pub(crate) fn top_commodities(city: &CityMarketData) -> Vec<&CommodityEntry> {
    let mut items: Vec<&CommodityEntry> = city.commodities.iter().collect();
    items.sort_by(|a, b| {
        b.arrivals
            .partial_cmp(&a.arrivals)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items.truncate(MAX_COMMODITIES_SHOWN);
    items
}

/// Instagram/YouTube-specific row/header builders: mirrors the columns in
/// `templates/ig_template_v2.html` (commodity, variety, grade, arrivals,
/// min, max -- no modal price), using the same sorted+capped commodity
/// list.
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
/// `templates/ig_template_v2.html`. The logo and header text are baked
/// directly into the `background_ig.png` asset (see `assets.rs`), so
/// this layout has no separate logo/header elements -- the content
/// starts with a `margin-bottom` spacer under the (invisible) header
/// area, matching the space reserved for the background's baked-in
/// branding. The background can be missing and the layout still
/// renders sensibly (falls back to the plain dark-green gradient).
pub fn instagram_html(
    city: &CityMarketData,
    report_date: &str,
    dict: &Dictionary,
    lang: Language,
    assets: &BrandingAssets,
) -> String {
    let city_name = dict.city_display(&city.city_name, lang);
    let report_date_label = dict.term("report_date", lang);
    let subscribe_label = dict.term("subscribe", lang);
    let watch_youtube_label = dict.term("watch_on_youtube", lang);

    let background_style = match &assets.background_ig_data_uri {
        Some(uri) => format!(
            "background-image:linear-gradient(rgba(6,59,6,.01),rgba(5,40,5,.01)),url('{}');background-size:cover;background-position:center;",
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
    background:linear-gradient(#063b0603,#05280503);
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
    margin-bottom:400px;
}}

.info-bar{{
    display:flex;
    margin:0 30px 20px;
    gap:20px;
    justify-content:center;
    align-content:center;
}}

.date-label{{
    background:rgba(33,95,23,.85);
    color:#fff;
    padding:16px;
    border-radius:12px;
    font-size:24px;
    font-weight:bold;
    align-content:center;
}}

.date-box{{
    color:#f3e80b;
    font-size:55px;
    font-weight:900;
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
    font-size:37px;
    font-weight:900;
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
    padding:25px 10px;
    border:1px solid #ddd;
    font-size:20px;
    font-weight:900;
}}

td{{
    padding:25px 10px;
    border:1px solid #ddd;
    text-align:center;
    font-size:18px;
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

/// Builds the branded YouTube (16:9) HTML cover for one city, mirroring
/// the Instagram v2 layout's visual language. The `background_yt.png`
/// asset (see `assets.rs`) already contains the logo/branding artwork
/// on the left; the info bar, market name and price table are confined
/// to a semi-transparent right-hand panel occupying 75% of the width so
/// the left 25% stays clear for the background art.
pub fn youtube_html(
    city: &CityMarketData,
    report_date: &str,
    dict: &Dictionary,
    lang: Language,
    assets: &BrandingAssets,
) -> String {
    let city_name = dict.city_display(&city.city_name, lang);
    let report_date_label = dict.term("report_date", lang);
    let subscribe_label = dict.term("subscribe", lang);
    let watch_youtube_label = dict.term("watch_on_youtube", lang);

    let background_style = match &assets.background_yt_data_uri {
        Some(uri) => format!(
            "background-image:linear-gradient(rgba(6,59,6,.01),rgba(5,40,5,.01)),url('{}');background-size:cover;background-position:center;",
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
    background:linear-gradient(#063b0603,#05280503);
    {background_style}
    color:#fff;
    display:flex;
    flex-direction:row;
}}

.spacer{{
    width:40%;
    flex-shrink:0;
}}

.content{{
    width:60%;
    flex-shrink:0;
    height:100%;
    display:flex;
    flex-direction:column;
    justify-content:center;
    gap:14px;
    padding:24px 30px;
}}

.info-bar{{
    display:flex;
    gap:14px;
    justify-content:center;
    align-content:center;
}}

.date-label{{
    background:rgba(33,95,23,.85);
    color:#fff;
    padding:10px 14px;
    border-radius:10px;
    font-size:16px;
    font-weight:bold;
    align-content:center;
}}

.date-box{{
    color:#f3e80b;
    font-size:26px;
    font-weight:900;
    display:flex;
    justify-content:center;
    align-items:center;
    padding:0 14px;
    border-radius:10px;
}}

.market{{
    width:100%;
    background:#d71e1e;
    color:#fff;
    text-align:center;
    padding:10px;
    border-radius:12px;
    font-size:22px;
    font-weight:900;
}}

table{{
    width:100%;
    border-collapse:collapse;
    background:#fff;
    color:#222;
}}

th{{
    background:#1c6d17;
    color:#fff;
    padding:8px;
    border:1px solid #ddd;
    font-size:13px;
    font-weight:900;
}}

td{{
    padding:6px;
    border:1px solid #ddd;
    text-align:center;
    font-size:12px;
}}

tbody tr:nth-child(even){{
    background:#f3fff0;
}}

.footer{{
    display:flex;
    flex-direction:column;
    align-items:center;
    gap:8px;
}}

.socials{{
    display:flex;
    align-items:center;
    gap:12px;
    flex-wrap:wrap;
    justify-content:center;
}}

.subscribe{{
    background:#e61d1d;
    color:#fff;
    padding:8px 22px;
    border-radius:50px;
    font-size:15px;
    font-weight:bold;
}}

.instagram-handle{{
    background:linear-gradient(45deg,#f58529,#dd2a7b,#8134af,#515bd4);
    color:#fff;
    padding:8px 16px;
    border-radius:50px;
    font-size:14px;
    font-weight:bold;
}}

.youtube-link{{
    color:#fff;
    font-size:12px;
    opacity:.9;
    word-break:break-all;
    text-align:center;
}}
</style>
</head>
<body>

<div class="container">

  <div class="spacer"></div>

  <div class="content">

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

</div>

</body>
</html>"#,
        lang_code = html_lang_code(lang),
        width = YOUTUBE_WIDTH,
        height = YOUTUBE_HEIGHT,
        background_style = background_style,
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
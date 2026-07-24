use anyhow::Result;
use scraper::{Html, Selector};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgriculturalReport {
    pub report_date: String,
    pub cities: Vec<CityMarketData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityMarketData {
    pub city_name: String,
    pub commodities: Vec<CommodityEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommodityEntry {
    pub commodity: String,
    pub variety: String,
    pub grade: String,
    #[serde(deserialize_with = "deserialize_number", default)]
    pub arrivals: f64,
    pub units: String,
    #[serde(deserialize_with = "deserialize_number", default)]
    pub min_rs: f64,
    #[serde(deserialize_with = "deserialize_number", default)]
    pub max_rs: f64,
    #[serde(deserialize_with = "deserialize_number", default)]
    pub modal_rs: f64,
}

pub fn parse_agricultural_report(html_string: &str) -> Result<AgriculturalReport> {
    let document = Html::parse_document(html_string);
    
    // Extract report date
    let date_selector = Selector::parse("span#_ctl0_MainContent_lbl_date").unwrap();
    let report_date = document
        .select(&date_selector)
        .next()
        .and_then(|el| el.text().next())
        .unwrap_or("Unknown")
        .to_string();
    
    // Parse all city data
    let mut cities = Vec::new();
    
    // Select all divs that contain city tables
    let div_selector = Selector::parse("td > div").unwrap();
    let span_selector = Selector::parse("span[style*='color:Red']").unwrap();
    
    let mut city_names = Vec::new();
    
    // Extract city names from red spans
    for span in document.select(&span_selector) {
        if let Some(text) = span.text().next() {
            let city_name = text.trim().to_string();
            city_names.push(city_name);
        }
    }
    
    // Extract commodity data from each div's table
    let mut city_index = 0;
    for div in document.select(&div_selector) {
        if city_index >= city_names.len() {
            break;
        }
        
        let table_selector = Selector::parse("table").unwrap();
        
        if let Some(table) = div.select(&table_selector).next() {
            let commodities = parse_commodity_table(table)?;
            
            if !commodities.is_empty() {
                cities.push(CityMarketData {
                    city_name: city_names[city_index].clone(),
                    commodities,
                });
                city_index += 1;
            }
        }
    }
    
    Ok(AgriculturalReport {
        report_date,
        cities,
    })
}

fn parse_commodity_table(table_element: scraper::element_ref::ElementRef) -> Result<Vec<CommodityEntry>> {
    let mut commodities = Vec::new();
    let row_selector = Selector::parse("tbody > tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();
    
    let mut is_header = true;
    
    for row in table_element.select(&row_selector) {
        if is_header {
            is_header = false;
            continue; // Skip header row
        }
        
        let cells: Vec<String> = row
            .select(&cell_selector)
            .map(|cell| cell.text().collect::<String>().trim().to_string())
            .collect();
        
        if cells.len() >= 8 {
            commodities.push(CommodityEntry {
                commodity: cells[0].clone(),
                variety: cells[1].clone(),
                grade: cells[2].clone(),
                arrivals: parse_number(&cells[3]),
                units: cells[4].clone(),
                min_rs: parse_number(&cells[5]),
                max_rs: parse_number(&cells[6]),
                modal_rs: parse_number(&cells[7]),
            });
        }
    }
    
    Ok(commodities)
}

fn parse_number(value: &str) -> f64 {
    value.replace(',', "").trim().parse::<f64>().unwrap_or(0.0)
}

fn deserialize_number<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    struct NumberVisitor;

    impl<'de> de::Visitor<'de> for NumberVisitor {
        type Value = f64;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a number, numeric string, or null")
        }

        fn visit_f64<E>(self, value: f64) -> Result<f64, E> {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<f64, E> {
            Ok(value as f64)
        }

        fn visit_u64<E>(self, value: u64) -> Result<f64, E> {
            Ok(value as f64)
        }

        fn visit_str<E>(self, value: &str) -> Result<f64, E> {
            Ok(parse_number(value))
        }

        fn visit_string<E>(self, value: String) -> Result<f64, E> {
            Ok(parse_number(&value))
        }

        fn visit_none<E>(self) -> Result<f64, E> {
            Ok(0.0)
        }

        fn visit_unit<E>(self) -> Result<f64, E> {
            Ok(0.0)
        }

        fn visit_bool<E>(self, _value: bool) -> Result<f64, E> {
            Ok(0.0)
        }
    }

    deserializer.deserialize_any(NumberVisitor)
}

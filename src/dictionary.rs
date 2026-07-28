use std::collections::HashMap;

/// Supported UI / output languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Kannada,
    English,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Kannada => "kannada",
            Language::English => "english",
        }
    }

    pub fn from_str(s: &str) -> Option<Language> {
        match s.to_lowercase().as_str() {
            "kannada" | "kn" => Some(Language::Kannada),
            "english" | "en" => Some(Language::English),
            _ => None,
        }
    }
}

/// A single city's English name plus Kannada translation.
/// `english` is always used for folder names / terminal display;
/// `kannada` is used for on-image text when Language::Kannada is selected.
#[derive(Debug, Clone)]
pub struct CityEntry {
    pub english: &'static str,
    pub kannada: &'static str,
}

/// A single commodity's English name, Kannada translation, and the
/// filename (without extension) of its representative image/icon.
#[derive(Debug, Clone)]
pub struct CommodityEntry {
    pub english: &'static str,
    pub kannada: &'static str,
    pub image_file: &'static str,
}

/// A single variety's English name plus Kannada translation.
/// Varieties are a separate category from commodities: a variety (e.g.
/// "Byadgi", "Local", "Hybrid") describes a cultivar/quality strain and
/// does NOT have its own dedicated image asset -- it reuses the parent
/// commodity's image.
#[derive(Debug, Clone)]
pub struct VarietyEntry {
    pub english: &'static str,
    pub kannada: &'static str,
}

/// A single grade's English name plus Kannada translation (e.g. FAQ,
/// Medium, Small).
#[derive(Debug, Clone)]
pub struct GradeEntry {
    pub english: &'static str,
    pub kannada: &'static str,
}

/// A single unit's English name plus Kannada translation (Quintal,
/// Numbers, Thousands).
#[derive(Debug, Clone)]
pub struct UnitEntry {
    pub english: &'static str,
    pub kannada: &'static str,
}

// -----------------------------------------------------------------------
// Data sets sourced from the Karnataka APMC daily market report (krama
// .karnataka.gov.in). Keys match the exact strings scraped from the
// source site so lookups succeed; add additional aliases in
// `normalize_key` as needed.
// -----------------------------------------------------------------------

pub fn city_dictionary() -> Vec<CityEntry> {
    vec![
        CityEntry { english: "ARSIKERE", kannada: "ಅರಸೀಕೆರೆ" },
        CityEntry { english: "BAGALKOT", kannada: "ಬಾಗಲಕೋಟೆ" },
        CityEntry { english: "BAGEPALLI", kannada: "ಬಾಗೇಪಲ್ಲಿ" },
        CityEntry { english: "BAILHONGAL", kannada: "ಬೈಲಹೊಂಗಲ" },
        CityEntry { english: "BANGARPET", kannada: "ಬಂಗಾರಪೇಟೆ" },
        CityEntry { english: "BANTWALA", kannada: "ಬಂಟ್ವಾಳ" },
        CityEntry { english: "BASAVAKALYANA", kannada: "ಬಸವಕಲ್ಯಾಣ" },
        CityEntry { english: "BELAGAVI", kannada: "ಬೆಳಗಾವಿ" },
        CityEntry { english: "BELTHANGADI", kannada: "ಬೆಳ್ತಂಗಡಿ" },
        CityEntry { english: "BELUR", kannada: "ಬೇಲೂರು" },
        CityEntry { english: "BENGALURU", kannada: "ಬೆಂಗಳೂರು" },
        CityEntry { english: "BHADRAVATHI", kannada: "ಭದ್ರಾವತಿ" },
        CityEntry { english: "BIDAR", kannada: "ಬೀದರ್" },
        CityEntry { english: "BINNY MILL (F&V)", kannada: "ಬಿನ್ನಿ ಮಿಲ್ (ಹಣ್ಣು ಮತ್ತು ತರಕಾರಿ)" },
        CityEntry { english: "C.R.NAGAR", kannada: "ಚಾಮರಾಜನಗರ" },
        CityEntry { english: "CHALLAKERE", kannada: "ಚಳ್ಳಕೆರೆ" },
        CityEntry { english: "CHANNAPATNA", kannada: "ಚನ್ನಪಟ್ಟಣ" },
        CityEntry { english: "CHIKKAMAGALURU", kannada: "ಚಿಕ್ಕಮಗಳೂರು" },
        CityEntry { english: "CHINCOLI", kannada: "ಚಿಂಚೋಳಿ" },
        CityEntry { english: "CHINTAMANI", kannada: "ಚಿಂತಾಮಣಿ" },
        CityEntry { english: "CHITRADURGA", kannada: "ಚಿತ್ರದುರ್ಗ" },
        CityEntry { english: "DAVANAGERE", kannada: "ದಾವಣಗೆರೆ" },
        CityEntry { english: "DODDABALLAPUR", kannada: "ದೊಡ್ಡಬಳ್ಳಾಪುರ" },
        CityEntry { english: "GANGAVATI", kannada: "ಗಂಗಾವತಿ" },
        CityEntry { english: "GONIKOPPAL", kannada: "ಗೋಣಿಕೊಪ್ಪಲು" },
        CityEntry { english: "GOWRIBIDNUR", kannada: "ಗೌರಿಬಿದನೂರು" },
        CityEntry { english: "GUNDLUPET", kannada: "ಗುಂಡ್ಲುಪೇಟೆ" },
        CityEntry { english: "HALIYALA", kannada: "ಹಳಿಯಾಳ" },
        CityEntry { english: "HARAPANAHALLI", kannada: "ಹರಪನಹಳ್ಳಿ" },
        CityEntry { english: "HARIHARA", kannada: "ಹರಿಹರ" },
        CityEntry { english: "HAVERI", kannada: "ಹಾವೇರಿ" },
        CityEntry { english: "HIRIYUR", kannada: "ಹಿರಿಯೂರು" },
        CityEntry { english: "HOLALKERE", kannada: "ಹೊಳಲ್ಕೆರೆ" },
        CityEntry { english: "HONNALI", kannada: "ಹೊನ್ನಾಳಿ" },
        CityEntry { english: "HOSANAGAR", kannada: "ಹೊಸನಗರ" },
    ]
}

pub fn commodity_dictionary() -> Vec<CommodityEntry> {
    vec![
        CommodityEntry { english: "Alasande Gram", kannada: "ಅಲಸಂದಿ ಬೇಳೆ / ಕಾಳು", image_file: "alasande_gram" },
        CommodityEntry { english: "Alasandikai", kannada: "ಅಲಸಂದಿಕಾಯಿ", image_file: "alasandikai" },
        CommodityEntry { english: "Apple", kannada: "ಸೇಬು", image_file: "apple" },
        CommodityEntry { english: "Arecanut", kannada: "ಅಡಿಕೆ", image_file: "arecanut" },
        CommodityEntry { english: "Bajra", kannada: "ಸಜ್ಜೆ", image_file: "bajra" },
        CommodityEntry { english: "Banana", kannada: "ಬಾಳೆಹಣ್ಣು", image_file: "banana" },
        CommodityEntry { english: "Banana - Green", kannada: "ಬಾಳೆಕಾಯಿ", image_file: "banana_green" },
        CommodityEntry { english: "Beans", kannada: "ಬೀನ್ಸ್", image_file: "beans" },
        CommodityEntry { english: "Beetroot", kannada: "ಬೀಟ್‌ರೂಟ್", image_file: "beetroot" },
        CommodityEntry { english: "Bengalgram", kannada: "ಕಡಲೆಕಾಳು", image_file: "bengalgram" },
        CommodityEntry { english: "Bengal Gramdal", kannada: "ಕಡಲೆಬೇಳೆ", image_file: "bengal_gramdal" },
        CommodityEntry { english: "Bitter Gourd", kannada: "ಹಾಗಲಕಾಯಿ", image_file: "bitter_gourd" },
        CommodityEntry { english: "Blackgram", kannada: "ಉದ್ದು", image_file: "blackgram" },
        CommodityEntry { english: "Black Gramdal", kannada: "ಉದ್ದಿನಬೇಳೆ", image_file: "black_gramdal" },
        CommodityEntry { english: "Bottle Gourd", kannada: "ಸೋರೆಕಾಯಿ", image_file: "bottle_gourd" },
        CommodityEntry { english: "Brinjal", kannada: "ಬದನೆಕಾಯಿ", image_file: "brinjal" },
        CommodityEntry { english: "Bunch Beans", kannada: "ಗುಂಪು ಬೀನ್ಸ್ / ತರಕಾರಿ ಬೀನ್ಸ್", image_file: "bunch_beans" },
        CommodityEntry { english: "Cabbage", kannada: "ಎಲೆಕೋಸು", image_file: "cabbage" },
        CommodityEntry { english: "Capsicum", kannada: "ಕ್ಯಾಪ್ಸಿಕಂ", image_file: "capsicum" },
        CommodityEntry { english: "Carrot", kannada: "ಕ್ಯಾರೇಟ್", image_file: "carrot" },
        CommodityEntry { english: "Cauliflower", kannada: "ಹೂಕೋಸು", image_file: "cauliflower" },
        CommodityEntry { english: "Chapparada Avare", kannada: "ಚಪ್ಪರದ ಅವರೆಕಾಯಿ", image_file: "chapparada_avare" },
        CommodityEntry { english: "Chennangidal", kannada: "ಚೆನ್ನಂಗಿ ಬೇಳೆ", image_file: "chennangidal" },
        CommodityEntry { english: "Chikoos(Sapota)", kannada: "ಸಪೋಟ", image_file: "chikoos_sapota" },
        CommodityEntry { english: "Chilly(Capsicum)", kannada: "ದೊಣ್ಣೆ ಮೆಣಸಿನಕಾಯಿ", image_file: "chilly_capsicum" },
        CommodityEntry { english: "Coco Brooms", kannada: "ತೆಂಗಿನ ಕಡ್ಡಿ (ಸೀಗು)", image_file: "coco_brooms" },
        CommodityEntry { english: "Coconut (Per 1000)", kannada: "ತೆಂಗಿನಕಾಯಿ (ಪ್ರತಿ 1000 ಕ್ಕೆ)", image_file: "coconut_per_1000" },
        CommodityEntry { english: "Copra", kannada: "ಕೊಬ್ಬರಿ", image_file: "copra" },
        CommodityEntry { english: "Coriander Seed", kannada: "ಧನಿಯಾ / ಕೊತ್ತಂಬರಿ ಬೀಜ", image_file: "coriander_seed" },
        CommodityEntry { english: "Cotton", kannada: "ಹತ್ತಿ", image_file: "cotton" },
        CommodityEntry { english: "Cowpea", kannada: "ಅಲಸಂದಿ ಕಾಳು", image_file: "cowpea" },
        CommodityEntry { english: "Cowpea(Veg)", kannada: "ಅಲಸಂದಿ ತರಕಾರಿ", image_file: "cowpea_veg" },
        CommodityEntry { english: "Cucumbar", kannada: "ಸೌತೆಕಾಯಿ", image_file: "cucumbar" },
        CommodityEntry { english: "Drum Stick", kannada: "ನುಗ್ಗೆಕಾಯಿ", image_file: "drum_stick" },
        CommodityEntry { english: "Dry Chillies", kannada: "ಒಣ ಮೆಣಸಿನಕಾಯಿ", image_file: "dry_chillies" },
        CommodityEntry { english: "Garlic", kannada: "ಬೆಳ್ಳುಳ್ಳಿ", image_file: "garlic" },
        CommodityEntry { english: "Grapes", kannada: "ದ್ರಾಕ್ಷಿ", image_file: "grapes" },
        CommodityEntry { english: "Green Avare (W)", kannada: "ಹಸಿ ಅವರೆ", image_file: "green_avare_w" },
        CommodityEntry { english: "Green Chilly", kannada: "ಹಸಿ ಮೆಣಸಿನಕಾಯಿ", image_file: "green_chilly" },
        CommodityEntry { english: "Green Ginger", kannada: "ಹಸಿ ಶುಂಟಿ", image_file: "green_ginger" },
        CommodityEntry { english: "Green Gramdal", kannada: "ಹೆಸರು ಬೇಳೆ", image_file: "green_gramdal" },
        CommodityEntry { english: "Green Peas", kannada: "ಹಸಿ ಬಟಾಣಿ", image_file: "green_peas" },
        CommodityEntry { english: "Greengram", kannada: "ಹೆಸರುಕಾಳು", image_file: "greengram" },
        CommodityEntry { english: "Groundnut", kannada: "ಕಡಲೆಕಾಯಿ", image_file: "groundnut" },
        CommodityEntry { english: "Groundnut Seed", kannada: "ಶೇಂಗಾ ಬೀಜ / ಕಡಲೆಕಾಯಿ ಬೀಜ", image_file: "groundnut_seed" },
        CommodityEntry { english: "Horse Gram", kannada: "ಹುರುಳಿ", image_file: "horse_gram" },
        CommodityEntry { english: "Jaggery", kannada: "ಬೆಲ್ಲ", image_file: "jaggery" },
        CommodityEntry { english: "Jowar", kannada: "ಜೋಳ", image_file: "jowar" },
        CommodityEntry { english: "Karbuja", kannada: "ಕರ್ಬೂಜ", image_file: "karbuja" },
        CommodityEntry { english: "Knool Khol", kannada: "ನವಿಲುಕೋಸು", image_file: "knool_khol" },
        CommodityEntry { english: "Ladies Finger", kannada: "ಬೆಂಡೆಕಾಯಿ", image_file: "ladies_finger" },
        CommodityEntry { english: "Lime (Lemon)", kannada: "ನಿಂಬೆಹಣ್ಣು", image_file: "lime_lemon" },
        CommodityEntry { english: "Lint", kannada: "ಹತ್ತಿ ನೂಲು", image_file: "lint" },
        CommodityEntry { english: "Maize", kannada: "ಮೆಕ್ಕೆಜೋಳ", image_file: "maize" },
        CommodityEntry { english: "Mango", kannada: "ಮಾಮಿನಹಣ್ಣು", image_file: "mango" },
        CommodityEntry { english: "Methi Seeds", kannada: "ಮೆಂತೆ ಬೀಜ", image_file: "methi_seeds" },
        CommodityEntry { english: "Moath", kannada: "ಮಟಕಿ / ತರಿ ಕಾಳು", image_file: "moath" },
        CommodityEntry { english: "Mousambi", kannada: "ಮೋಸಂಬಿ", image_file: "mousambi" },
        CommodityEntry { english: "Mustard", kannada: "ಸಾಸಿವೆ", image_file: "mustard" },
        CommodityEntry { english: "Navane", kannada: "ನವಣೆ", image_file: "navane" },
        CommodityEntry { english: "Neem Seed", kannada: "ಬೇಪಿನ ಬೀಜ", image_file: "neem_seed" },
        CommodityEntry { english: "Onion", kannada: "ಈರುಳ್ಳಿ", image_file: "onion" },
        CommodityEntry { english: "Paddy", kannada: "ನೆಲ್ಲು / ಭತ್ತ", image_file: "paddy" },
        CommodityEntry { english: "Papaya", kannada: "ಪಪ್ಪಾಯಿ", image_file: "papaya" },
        CommodityEntry { english: "Peas(Wet)", kannada: "ಹಸಿ ಬಟಾಣಿ", image_file: "peas_wet" },
        CommodityEntry { english: "Pepper", kannada: "ಕಾಳುಮೆಣಸು", image_file: "pepper" },
        CommodityEntry { english: "Pine Apple", kannada: "ಅನಾನಸ್", image_file: "pine_apple" },
        CommodityEntry { english: "Pomagranate", kannada: "ದಾಳಿಂಬೆ", image_file: "pomagranate" },
        CommodityEntry { english: "Potato", kannada: "ಆಲೂಗಡ್ಡೆ", image_file: "potato" },
        CommodityEntry { english: "Raddish", kannada: "ಮುಲ್ಲಂಗಿ", image_file: "raddish" },
        CommodityEntry { english: "Ragi", kannada: "ರಾಗಿ", image_file: "ragi" },
        CommodityEntry { english: "Rice", kannada: "ಅಕ್ಕಿ", image_file: "rice" },
        CommodityEntry { english: "Ridgeguard", kannada: "ಹೀರೇಕಾಯಿ", image_file: "ridgeguard" },
        CommodityEntry { english: "Safflower", kannada: "ಕುಸುಬೆ", image_file: "safflower" },
        CommodityEntry { english: "Seemebadanekai", kannada: "ಸೀಮೆಬದನೆಕಾಯಿ", image_file: "seemebadanekai" },
        CommodityEntry { english: "Snakeguard", kannada: "ಪಡವಲಕಾಯಿ", image_file: "snakeguard" },
        CommodityEntry { english: "Soyabeen", kannada: "ಸೋಯಾಬೀನ್", image_file: "soyabeen" },
        CommodityEntry { english: "Sunflower", kannada: "ಸೂರ್ಯಕಾಂತಿ", image_file: "sunflower" },
        CommodityEntry { english: "Suvarnagadde", kannada: "ಸುವರ್ಣಗಡ್ಡೆ", image_file: "suvarnagadde" },
        CommodityEntry { english: "Sweet Potato", kannada: "ಗೆಣಸು", image_file: "sweet_potato" },
        CommodityEntry { english: "Sweet Pumpkin", kannada: "ಸಿಹಿ ಕುಂಬಳಕಾಯಿ", image_file: "sweet_pumpkin" },
        CommodityEntry { english: "Tender Coconut", kannada: "ಎಳನೀರು", image_file: "tender_coconut" },
        CommodityEntry { english: "Thondekai", kannada: "ತೊಂಡೇಕಾಯಿ", image_file: "thondekai" },
        CommodityEntry { english: "Tomato", kannada: "ಟೊಮೆಟೊ", image_file: "tomato" },
        CommodityEntry { english: "Tur", kannada: "ತೊಗರಿ", image_file: "tur" },
        CommodityEntry { english: "Tur Dal", kannada: "ತೊಗರಿ ಬೇಳೆ", image_file: "tur_dal" },
        CommodityEntry { english: "Water Melon", kannada: "ಕಲ್ಲಂಗಡಿ", image_file: "water_melon" },
        CommodityEntry { english: "Wheat", kannada: "ಗೋಧಿ", image_file: "wheat" },
        CommodityEntry { english: "White Pumpkin", kannada: "ಬೂದು ಕುಂಬಳಕಾಯಿ", image_file: "white_pumpkin" },
    ]
}

/// Varieties are keyed and looked up independently of commodities: the
/// same variety name (e.g. "Local", "Hybrid") can apply across many
/// different commodities, and a (commodity, variety) pair does not map
/// to a unique image -- only the commodity does.
pub fn variety_dictionary() -> Vec<VarietyEntry> {
    vec![
        VarietyEntry { english: "Alasande Gram", kannada: "ಅಲಸಂದಿ ಬೇಳೆ" },
        VarietyEntry { english: "Alasandikai", kannada: "ಅಲಸಂದಿಕಾಯಿ" },
        VarietyEntry { english: "Apple", kannada: "ಸೇಬು" },
        VarietyEntry { english: "Average (Whole)", kannada: "ಸಾಧಾರಣ (ಇಡೀ ಕಾಳು)" },
        VarietyEntry { english: "Bangalore Small", kannada: "ಬೆಂಗಳೂರು ಸಣ್ಣ ಈರುಳ್ಳಿ" },
        VarietyEntry { english: "Banana - Green (Balekai)", kannada: "ಬಾಳೆಕಾಯಿ" },
        VarietyEntry { english: "Beans (Whole)", kannada: "ಬೀನ್ಸ್ (ಇಡೀ)" },
        VarietyEntry { english: "Beetroot", kannada: "ಬೀಟ್‌ರೂಟ್" },
        VarietyEntry { english: "Bitter Gourd", kannada: "ಹಾಗಲಕಾಯಿ" },
        VarietyEntry { english: "Black", kannada: "ಕಪ್ಪು" },
        VarietyEntry { english: "Black Pepper", kannada: "ಕಪ್ಪು ಕಾಳುಮೆಣಸು" },
        VarietyEntry { english: "Black Gram Dal", kannada: "ಉದ್ದಿನ ಬೇಳೆ" },
        VarietyEntry { english: "Black Gram (Whole)", kannada: "ಇಡೀ ಉದ್ದು" },
        VarietyEntry { english: "Bottle Gourd", kannada: "ಸೋರೆಕಾಯಿ" },
        VarietyEntry { english: "Brinjal", kannada: "ಬದನೆಕಾಯಿ" },
        VarietyEntry { english: "Broken Rice", kannada: "ನುಚ್ಚು ಅಕ್ಕಿ" },
        VarietyEntry { english: "Bunch Beans", kannada: "ಗುಂಪು ಬೀನ್ಸ್" },
        VarietyEntry { english: "Byadgi", kannada: "ಬ್ಯಾಡಗಿ" },
        VarietyEntry { english: "Cabbage", kannada: "ಎಲೆಕೋಸು" },
        VarietyEntry { english: "Capsicum", kannada: "ಕ್ಯಾಪ್ಸಿಕಂ" },
        VarietyEntry { english: "Carrot", kannada: "ಕ್ಯಾರೇಟ್" },
        VarietyEntry { english: "Cauliflower", kannada: "ಹೂಕೋಸು" },
        VarietyEntry { english: "Chapparada Avarekai", kannada: "ಚಪ್ಪರದ ಅವರೆಕಾಯಿ" },
        VarietyEntry { english: "Chennagidal", kannada: "ಚೆನ್ನಂಗಿ ಬೇಳೆ" },
        VarietyEntry { english: "Chilly(Capsicum)", kannada: "ದೊಣ್ಣೆ ಮೆಣಸಿನಕಾಯಿ" },
        VarietyEntry { english: "Coarse", kannada: "ದಪ್ಪ / ದಪ್ಪ ತಳಿ" },
        VarietyEntry { english: "Coca", kannada: "ಕೋಕಾ" },
        VarietyEntry { english: "Coco Brooms", kannada: "ತೆಂಗಿನ ಕಡ್ಡಿ (ಸೀಗು)" },
        VarietyEntry { english: "Coconut", kannada: "ತೆಂಗಿನಕಾಯಿ" },
        VarietyEntry { english: "Coriander Seed", kannada: "ಕೊತ್ತಂಬರಿ ಬೀಜ" },
        VarietyEntry { english: "Cowpea (Veg)", kannada: "ಅಲಸಂದಿ ತರಕಾರಿ" },
        VarietyEntry { english: "Cowpea (Whole)", kannada: "ಕೌಪಿಯಾ (ವೋಲ್) / ಇಡೀ ಅಲಸಂದಿ" },
        VarietyEntry { english: "Cucumbar", kannada: "ಸೌತೆಕಾಯಿ" },
        VarietyEntry { english: "DCH", kannada: "ಡಿ.ಸಿ.ಹೆಚ್" },
        VarietyEntry { english: "Dappa", kannada: "ದಪ್ಪ" },
        VarietyEntry { english: "Drumstick", kannada: "ನುಗ್ಗೆಕಾಯಿ" },
        VarietyEntry { english: "Elakki Bale", kannada: "ಏಲಕ್ಕಿ ಬಾಳೆ" },
        VarietyEntry { english: "FAQ", kannada: "ಎಫ್.ಎ.ಕ್ಯೂ (ಸಾಧಾರಣ ತಳಿ)" },
        VarietyEntry { english: "Fine", kannada: "ಸಣ್ಣ / ಉತ್ತಮ ತಳಿ" },
        VarietyEntry { english: "Garlic", kannada: "ಬೆಳ್ಳುಳ್ಳಿ" },
        VarietyEntry { english: "Gejje", kannada: "ಗೆಜ್ಜೆ" },
        VarietyEntry { english: "Green Avare (W)", kannada: "ಹಸಿ ಅವರೆ" },
        VarietyEntry { english: "Green Chilly", kannada: "ಹಸಿ ಮೆಣಸಿನಕಾಯಿ" },
        VarietyEntry { english: "Green Ginger", kannada: "ಹಸಿ ಶುಂಟಿ" },
        VarietyEntry { english: "Green Gram Dal", kannada: "ಹೆಸರು ಬೇಳೆ" },
        VarietyEntry { english: "Green Gram (Whole)", kannada: "ಹೆಸರುಕಾಳು" },
        VarietyEntry { english: "Green Peas", kannada: "ಹಸಿ ಬಟಾಣಿ" },
        VarietyEntry { english: "Ground Nut Seed", kannada: "ಕಡಲೆಕಾಯಿ ಬೀಜ" },
        VarietyEntry { english: "Guntur", kannada: "ಗುಂಟೂರು" },
        VarietyEntry { english: "Horse Gram (Whole)", kannada: "ಇಡೀ ಹುರುಳಿ" },
        VarietyEntry { english: "Hybrid", kannada: "ಹೈಬ್ರಿಡ್" },
        VarietyEntry { english: "Hybrid/Local", kannada: "ಹೈಬ್ರಿಡ್ / ಸ್ಥಳೀಯ" },
        VarietyEntry { english: "Hybrid-44", kannada: "ಹೈಬ್ರಿಡ್-44" },
        VarietyEntry { english: "IR-64", kannada: "ಐ.ಆರ್-64" },
        VarietyEntry { english: "Jawari/Local", kannada: "ಜವಾರಿ / ಸ್ಥಳೀಯ" },
        VarietyEntry { english: "Jowar (White)", kannada: "ಬಿಳಿ ಜೋಳ" },
        VarietyEntry { english: "Karbhuja", kannada: "ಕರ್ಬೂಜ" },
        VarietyEntry { english: "Kaveri Sona", kannada: "ಕಾವೇರಿ ಸೋನಾ" },
        VarietyEntry { english: "Kempugotu", kannada: "ಕೆಂಪುಗೋಟು" },
        VarietyEntry { english: "Knool Khol", kannada: "ನವಿಲುಕೋಸು" },
        VarietyEntry { english: "Ladies Finger", kannada: "ಬೆಂಡೆಕಾಯಿ" },
        VarietyEntry { english: "Lime (Lemon)", kannada: "ನಿಂಬೆಹಣ್ಣು" },
        VarietyEntry { english: "Local", kannada: "ಸ್ಥಳೀಯ (ನಾಟಿ)" },
        VarietyEntry { english: "Medium", kannada: "ಮಧ್ಯಮ ತಳಿ" },
        VarietyEntry { english: "Methiseeds", kannada: "ಮೆಂತೆ ಬೀಜ" },
        VarietyEntry { english: "Mexican", kannada: "ಮೆಕ್ಸಿಕನ್" },
        VarietyEntry { english: "Mill Wheat", kannada: "ಮಿಲ್ ಗೋಧಿ" },
        VarietyEntry { english: "Milling", kannada: "ಮಿಲ್ಲಿಂಗ್ ಕೊಬ್ಬರಿ" },
        VarietyEntry { english: "Moath (W)", kannada: "ಇಡೀ ಮಟಕಿ" },
        VarietyEntry { english: "Mudde", kannada: "ಮುದ್ದೆ ಬೆಲ್ಲ" },
        VarietyEntry { english: "Nauti Bale", kannada: "ನಾಟಿ ಬಾಳೆ" },
        VarietyEntry { english: "Neelam", kannada: "ನೀಲಂ" },
        VarietyEntry { english: "Neem Seed", kannada: "ಬೇಪಿನ ಬೀಜ" },
        VarietyEntry { english: "Nendra Bale", kannada: "ನೇಂದ್ರ ಬಾಳೆ" },
        VarietyEntry { english: "New Variety", kannada: "ಹೊಸ ತಳಿ" },
        VarietyEntry { english: "Onion", kannada: "ಈರುಳ್ಳಿ" },
        VarietyEntry { english: "Other", kannada: "ಇತರೆ" },
        VarietyEntry { english: "Others", kannada: "ಇತರ ತಳಿಗಳು" },
        VarietyEntry { english: "Pachha Bale", kannada: "ಪಚ್ಚ ಬಾಳೆ" },
        VarietyEntry { english: "Paddy", kannada: "ಭತ್ತ" },
        VarietyEntry { english: "Paddy 1001", kannada: "ಭತ್ತ 1001" },
        VarietyEntry { english: "Paddy Medium Variety", kannada: "ಮಧ್ಯಮ ತಳಿಯ ಭತ್ತ" },
        VarietyEntry { english: "Paddy RNR (New)", kannada: "ಆರ್.ಎನ್.ಆರ್ ಹೊಸ ಭತ್ತ" },
        VarietyEntry { english: "Paddy RNR (Old)", kannada: "ಆರ್.ಎನ್.ಆರ್ ಹಳೆಯ ಭತ್ತ" },
        VarietyEntry { english: "Papaya", kannada: "ಪಪ್ಪಾಯಿ" },
        VarietyEntry { english: "Peas(Wet)", kannada: "ಹಸಿ ಬಟಾಣಿ" },
        VarietyEntry { english: "Pine Apple", kannada: "ಅನಾನಸ್" },
        VarietyEntry { english: "Pomogranate", kannada: "ದಾಳಿಂಬೆ" },
        VarietyEntry { english: "Potato", kannada: "ಆಲೂಗಡ್ಡೆ" },
        VarietyEntry { english: "Puna", kannada: "ಪೂನಾ ಈರುಳ್ಳಿ" },
        VarietyEntry { english: "Raddish", kannada: "ಮುಲ್ಲಂಗಿ" },
        VarietyEntry { english: "Rashi", kannada: "ರಾಶಿ" },
        VarietyEntry { english: "Ridgeguard", kannada: "ಹೀರೇಕಾಯಿ" },
        VarietyEntry { english: "Round", kannada: "ಉಂಡೆ / ಉರುಟು" },
        VarietyEntry { english: "Round/Long", kannada: "ಉಂಡೆ / ಉದ್ದ" },
        VarietyEntry { english: "Safflower", kannada: "ಕುಸುಬೆ" },
        VarietyEntry { english: "Sapota", kannada: "ಸಪೋಟ" },
        VarietyEntry { english: "Seemebadanekai", kannada: "ಸೀಮೆಬದನೆಕಾಯಿ" },
        VarietyEntry { english: "Sippegotu", kannada: "ಸಿಪ್ಪೆಗೋಟು" },
        VarietyEntry { english: "Snakeguard", kannada: "ಪಡವಲಕಾಯಿ" },
        VarietyEntry { english: "Soyabeen", kannada: "ಸೋಯಾಬೀನ್" },
        VarietyEntry { english: "Sunflower", kannada: "ಸೂರ್ಯಕಾಂತಿ" },
        VarietyEntry { english: "Suvarnagadde", kannada: "ಸುವರ್ಣಗಡ್ಡೆ" },
        VarietyEntry { english: "Sweet Potato", kannada: "ಗೆಣಸು" },
        VarietyEntry { english: "Sweet Pumpkin", kannada: "ಸಿಹಿ ಕುಂಬಳ" },
        VarietyEntry { english: "Tender Coconut", kannada: "ಎಳನೀರು" },
        VarietyEntry { english: "Thondekai", kannada: "ತೊಂಡೇಕಾಯಿ" },
        VarietyEntry { english: "Tomato", kannada: "ಟೊಮೆಟೊ" },
        VarietyEntry { english: "Totapuri", kannada: "ತೋತಾಪುರಿ" },
        VarietyEntry { english: "Tur", kannada: "ತೊಗರಿ" },
        VarietyEntry { english: "Tur Dal", kannada: "ತೊಗರಿ ಬೇಳೆ" },
        VarietyEntry { english: "Unde", kannada: "ಉಂಡೆ ಬೆಲ್ಲ" },
        VarietyEntry { english: "Water Melon", kannada: "ಕಲ್ಲಂಗಡಿ" },
        VarietyEntry { english: "White", kannada: "ಬಿಳಿ" },
        VarietyEntry { english: "White Pumpkin", kannada: "ಬೂದು ಕುಂಬಳ" },
        VarietyEntry { english: "Yellow", kannada: "ಹಳದಿ" },
    ]
}

pub fn grade_dictionary() -> Vec<GradeEntry> {
    vec![
        GradeEntry { english: "Average", kannada: "ಸಾಧಾರಣ" },
        GradeEntry { english: "FAQ", kannada: "ಎಫ್.ಎ.ಕ್ಯೂ (ಉತ್ತಮ ಗುಣಮಟ್ಟ)" },
        GradeEntry { english: "Large", kannada: "ದೊಡ್ಡದು / ಉತ್ತಮ ಶ್ರೇಣಿ" },
        GradeEntry { english: "Medium", kannada: "ಮಧ್ಯಮ" },
        GradeEntry { english: "Small", kannada: "ಸಣ್ಣದು" },
    ]
}

pub fn unit_dictionary() -> Vec<UnitEntry> {
    vec![
        UnitEntry { english: "Numbers", kannada: "ಸಂಖ್ಯೆಗಳು" },
        UnitEntry { english: "Quintal", kannada: "ಕ್ವಿಂಟಾಲ್" },
        UnitEntry { english: "Thousands", kannada: "ಸಾವಿರಗಳು" },
    ]
}

/// Common UI / template terms: quintal, table headers, subscribe,
/// daily rate, weekdays list, etc.
pub fn common_terms() -> HashMap<&'static str, (&'static str, &'static str)> {
    // key -> (english, kannada)
    let mut m = HashMap::new();
    m.insert("quintal", ("Quintal", "ಕ್ವಿಂಟಾಲ್"));
    m.insert("commodity", ("Commodity", "ಸರಕು"));
    m.insert("variety", ("Variety", "ತಳಿ"));
    m.insert("grade", ("Grade", "ದರ್ಜೆ"));
    m.insert("arrivals", ("Arrivals", "ಆಗಮನ"));
    m.insert("units", ("Units", "ಘಟಕ"));
    m.insert("min_price", ("Min Price", "ಕನಿಷ್ಠ ಬೆಲೆ"));
    m.insert("max_price", ("Max Price", "ಗರಿಷ್ಠ ಬೆಲೆ"));
    m.insert("modal_price", ("Modal Price", "ಸಾಮಾನ್ಯ ಬೆಲೆ"));
    m.insert("daily_rate", ("Daily Rate", "ದೈನಂದಿನ ದರ"));
    m.insert("report_date", ("Report Date", "ವರದಿ ದಿನಾಂಕ"));
    m.insert("subscribe", ("Subscribe", "ಚಂದಾದಾರರಾಗಿ"));
    m.insert("title", ("Daily Agricultural Report", "ಉತ್ಪನ್ನವಾರು ದೈನಂದಿನ ವರದಿ"));
    m.insert("all_cities", ("All Cities", "ಎಲ್ಲಾ ನಗರಗಳು"));
    m.insert("follow_instagram", ("Follow us on Instagram", "ಇನ್‌ಸ್ಟಾಗ್ರಾಮ್‌ನಲ್ಲಿ ಫಾಲೋ ಮಾಡಿ"));
    m.insert("watch_on_youtube", ("Watch on YouTube", "ಯೂಟ್ಯೂಬ್‌ನಲ್ಲಿ ವೀಕ್ಷಿಸಿ"));
    m
}

pub fn weekdays() -> HashMap<&'static str, (&'static str, &'static str)> {
    // chrono weekday name (English, as returned by %A) -> (english display, kannada)
    let mut m = HashMap::new();
    m.insert("Monday", ("Monday", "ಸೋಮವಾರ"));
    m.insert("Tuesday", ("Tuesday", "ಮಂಗಳವಾರ"));
    m.insert("Wednesday", ("Wednesday", "ಬುಧವಾರ"));
    m.insert("Thursday", ("Thursday", "ಗುರುವಾರ"));
    m.insert("Friday", ("Friday", "ಶುಕ್ರವಾರ"));
    m.insert("Saturday", ("Saturday", "ಶನಿವಾರ"));
    m.insert("Sunday", ("Sunday", "ಭಾನುವಾರ"));
    m
}

/// Lookup tables built from the vectors above, keyed by normalized
/// (lowercased, trimmed) English name for fast bidirectional lookup.
pub struct Dictionary {
    pub cities_by_en: HashMap<String, CityEntry>,
    pub commodities_by_en: HashMap<String, CommodityEntry>,
    pub varieties_by_en: HashMap<String, VarietyEntry>,
    pub grades_by_en: HashMap<String, GradeEntry>,
    pub units_by_en: HashMap<String, UnitEntry>,
    pub terms: HashMap<&'static str, (&'static str, &'static str)>,
    pub weekdays: HashMap<&'static str, (&'static str, &'static str)>,
}

impl Dictionary {
    pub fn load() -> Self {
        let mut cities_by_en = HashMap::new();
        for c in city_dictionary() {
            cities_by_en.insert(normalize_key(c.english), c);
        }

        let mut commodities_by_en = HashMap::new();
        for c in commodity_dictionary() {
            commodities_by_en.insert(normalize_key(c.english), c);
        }

        let mut varieties_by_en = HashMap::new();
        for v in variety_dictionary() {
            varieties_by_en.insert(normalize_key(v.english), v);
        }

        let mut grades_by_en = HashMap::new();
        for g in grade_dictionary() {
            grades_by_en.insert(normalize_key(g.english), g);
        }

        let mut units_by_en = HashMap::new();
        for u in unit_dictionary() {
            units_by_en.insert(normalize_key(u.english), u);
        }

        Dictionary {
            cities_by_en,
            commodities_by_en,
            varieties_by_en,
            grades_by_en,
            units_by_en,
            terms: common_terms(),
            weekdays: weekdays(),
        }
    }

    /// Translate a scraped city name (as found on the source site, in
    /// either language) to its canonical English form used for folder
    /// names and terminal display. Falls back to the input if unknown.
    pub fn city_to_english<'a>(&'a self, name: &'a str) -> String {
        let key = normalize_city_key(name);
        if let Some(entry) = self.cities_by_en.get(&key) {
            return entry.english.to_string();
        }
        // Try matching against Kannada names too.
        let stripped = strip_city_prefix(name);
        for entry in self.cities_by_en.values() {
            if entry.kannada == stripped {
                return entry.english.to_string();
            }
        }
        stripped
    }

    /// Display a city name in the requested language, defaulting to the
    /// raw scraped value if not found in the dictionary.
    pub fn city_display(&self, name: &str, lang: Language) -> String {
        let key = normalize_city_key(name);
        if let Some(entry) = self.cities_by_en.get(&key) {
            return match lang {
                Language::English => entry.english.to_string(),
                Language::Kannada => entry.kannada.to_string(),
            };
        }
        strip_city_prefix(name)
    }

    pub fn commodity_display(&self, name: &str, lang: Language) -> String {
        let key = normalize_key(name);
        if let Some(entry) = self.commodities_by_en.get(&key) {
            return match lang {
                Language::English => entry.english.to_string(),
                Language::Kannada => entry.kannada.to_string(),
            };
        }
        name.trim().to_string()
    }

    /// Image filename (no extension) for a commodity, used to look up
    /// the packaged icon/background asset. Falls back to a normalized
    /// slug of the English name if not found. Note: varieties do NOT
    /// have their own image -- always look up the image by commodity.
    pub fn commodity_image_file(&self, name: &str) -> String {
        let key = normalize_key(name);
        if let Some(entry) = self.commodities_by_en.get(&key) {
            return entry.image_file.to_string();
        }
        key.replace(' ', "_")
    }

    /// Display a variety name in the requested language, defaulting to
    /// the raw scraped value if not found in the dictionary.
    pub fn variety_display(&self, name: &str, lang: Language) -> String {
        let key = normalize_key(name);
        if let Some(entry) = self.varieties_by_en.get(&key) {
            return match lang {
                Language::English => entry.english.to_string(),
                Language::Kannada => entry.kannada.to_string(),
            };
        }
        name.trim().to_string()
    }

    /// Display a grade name in the requested language, defaulting to
    /// the raw scraped value if not found in the dictionary.
    pub fn grade_display(&self, name: &str, lang: Language) -> String {
        let key = normalize_key(name);
        if let Some(entry) = self.grades_by_en.get(&key) {
            return match lang {
                Language::English => entry.english.to_string(),
                Language::Kannada => entry.kannada.to_string(),
            };
        }
        name.trim().to_string()
    }

    /// Display a unit name in the requested language, defaulting to
    /// the raw scraped value if not found in the dictionary.
    pub fn unit_display(&self, name: &str, lang: Language) -> String {
        let key = normalize_key(name);
        if let Some(entry) = self.units_by_en.get(&key) {
            return match lang {
                Language::English => entry.english.to_string(),
                Language::Kannada => entry.kannada.to_string(),
            };
        }
        name.trim().to_string()
    }

    pub fn term(&self, key: &str, lang: Language) -> String {
        match self.terms.get(key) {
            Some((en, kn)) => match lang {
                Language::English => en.to_string(),
                Language::Kannada => kn.to_string(),
            },
            None => key.to_string(),
        }
    }

    pub fn weekday(&self, chrono_name: &str, lang: Language) -> String {
        match self.weekdays.get(chrono_name) {
            Some((en, kn)) => match lang {
                Language::English => en.to_string(),
                Language::Kannada => kn.to_string(),
            },
            None => chrono_name.to_string(),
        }
    }

    /// All known city display names for the given language, sorted.
    pub fn all_city_names(&self, lang: Language) -> Vec<String> {
        let mut names: Vec<String> = self
            .cities_by_en
            .values()
            .map(|c| match lang {
                Language::English => c.english.to_string(),
                Language::Kannada => c.kannada.to_string(),
            })
            .collect();
        names.sort();
        names
    }
}

/// Folder-safe English name: spaces -> underscores.
pub fn city_folder_name(english_name: &str) -> String {
    english_name.trim().replace(' ', "_")
}

/// Strips the report's city-name prefix, e.g. `"11]   MARKET: BENGALURU"`
/// -> `"BENGALURU"`, or `"29]  ಮಾರುಕಟ್ಟೆ: ಬೆಂಗಳೂರು"` -> `"ಬೆಂಗಳೂರು"`.
///
/// The source site prefixes every city name with a serial number in
/// square brackets followed by a "MARKET:" (English) or "ಮಾರುಕಟ್ಟೆ:"
/// (Kannada) label. Only the text after the final colon is the actual
/// city name; everything before it must be stripped before dictionary
/// lookup.
pub fn strip_city_prefix(raw: &str) -> String {
    let s = raw.trim();
    // Take everything after the last ':' if present (covers both the
    // English "MARKET:" and Kannada "ಮಾರುಕಟ್ಟೆ:" labels, and is robust
    // to any other label wording the site might use).
    if let Some(idx) = s.rfind(':') {
        s[idx + 1..].trim().to_string()
    } else {
        s.to_string()
    }
}

fn normalize_key(s: &str) -> String {
    s.trim().to_lowercase()
}

fn normalize_city_key(s: &str) -> String {
    strip_city_prefix(s).to_lowercase()
}
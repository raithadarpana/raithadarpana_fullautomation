//! Reusable caption/title text shared by every social-media publisher.
//!
//! Both Instagram (Reel caption) and YouTube (title + description) use
//! the exact same formatted string, so it is built in exactly one place
//! to avoid the two formats drifting apart.

/// Builds the caption/title/description text used across all platforms:
///
/// `{market} ಮಾರುಕಟ್ಟೆ ದರಗಳು {date} 🌱 ರೈತ ದರ್ಪಣ #farming #farmer #apmc`
///
/// `market` should be the Kannada display name of the selected market
/// (e.g. "ರಾಮನಗರ") and `date` the selected report date formatted as
/// `dd-mm-yyyy`. Neither value is hard-coded here; both are supplied by
/// the caller from the application's selected-market / selected-date
/// state.
pub fn build_social_caption(market: &str, date: &str) -> String {
    format!("{market} ಮಾರುಕಟ್ಟೆ ದರಗಳು {date} 🌱 ರೈತ ದರ್ಪಣ #farming #farmer #apmc")
}

/// The fixed tag set requested for every YouTube upload. Kept as a
/// function (rather than a `const`) so it can be swapped out for a
/// config-driven list later, per the "make tags configurable in the
/// future" requirement, without changing call sites.
pub fn youtube_tags() -> Vec<String> {
    [
        "apmc",
        "farmer",
        "marketupdate",
        "dailymarketupdate",
        "farming",
        "farmers life",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// Formats a `YYYYMMDD` storage-style date (as used elsewhere in this
/// project, see `storage::day_dir`) into the `dd-mm-yyyy` form used in
/// captions/titles. Falls back to returning the input unchanged if it
/// isn't 8 digits, rather than panicking on unexpected input.
pub fn format_caption_date(date_ymd: &str) -> String {
    if date_ymd.len() == 8 && date_ymd.chars().all(|c| c.is_ascii_digit()) {
        let (y, rest) = date_ymd.split_at(4);
        let (m, d) = rest.split_at(2);
        format!("{d}-{m}-{y}")
    } else {
        date_ymd.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_caption() {
        let caption = build_social_caption("ರಾಮನಗರ", "17-08-2026");
        assert_eq!(
            caption,
            "ರಾಮನಗರ ಮಾರುಕಟ್ಟೆ ದರಗಳು 17-08-2026 🌱 ರೈತ ದರ್ಪಣ #farming #farmer #apmc"
        );
    }

    #[test]
    fn caption_uses_supplied_market_not_a_hardcoded_one() {
        let caption = build_social_caption("ಬೆಂಗಳೂರು", "01-01-2027");
        assert!(caption.starts_with("ಬೆಂಗಳೂರು "));
        assert!(!caption.contains("ರಾಮನಗರ"));
    }

    #[test]
    fn youtube_tags_contains_required_tags() {
        let tags = youtube_tags();
        for expected in [
            "apmc",
            "farmer",
            "marketupdate",
            "dailymarketupdate",
            "farming",
            "farmers life",
        ] {
            assert!(tags.iter().any(|t| t == expected), "missing tag: {expected}");
        }
    }

    #[test]
    fn formats_ymd_date_for_captions() {
        assert_eq!(format_caption_date("20260817"), "17-08-2026");
    }

    #[test]
    fn leaves_unrecognized_date_untouched() {
        assert_eq!(format_caption_date("17/08/2026"), "17/08/2026");
    }
}
//! Choosing the right poster from IMDb title suggestions.

use serde_json::Value;

/// Percent-encode a search query for the suggestion URL.
pub fn encode(q: &str) -> String {
    let mut out = String::new();
    for b in q.as_bytes() {
        match *b {
            b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' => out.push(*b as char),
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

fn simplify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|w| !["the", "a", "an"].contains(w))
        .collect::<Vec<_>>()
        .join(" ")
}

/// How well a suggestion matches; 4 or more is a confident match.
pub fn score(cand: &Value, title: &str, year: Option<i32>, show: bool) -> i32 {
    let l = cand["l"].as_str().unwrap_or("");
    if cand["i"]["imageUrl"].as_str().is_none() {
        return -100;
    }
    let (a, b) = (simplify(l), simplify(title));
    let mut s = if a == b {
        4
    } else if !b.is_empty() && (a.starts_with(&b) || b.starts_with(&a)) {
        1
    } else {
        -10
    };
    let qid = cand["qid"].as_str().unwrap_or("");
    let is_show = matches!(qid, "tvSeries" | "tvMiniSeries");
    let is_movie = matches!(qid, "movie" | "tvMovie" | "video");
    if (show && is_show) || (!show && is_movie) {
        s += 2;
    } else if show != is_show {
        s -= 2;
    }
    if let (Some(y), Some(cy)) = (year, cand["y"].as_i64()) {
        let d = (y as i64 - cy).abs();
        s += match d {
            0 => 3,
            1 => 1,
            _ if !show => -3,
            _ => 0,
        };
    }
    s
}

/// Ask the image CDN for a 400 px wide poster instead of the original.
pub fn resized(url: &str) -> String {
    if let Some(i) = url.rfind("._V1_") {
        format!("{}._V1_QL80_UX400_.jpg", &url[..i])
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn scoring_prefers_exact_title_year_and_type() {
        let m = json!({"l":"The Matrix","y":1999,"qid":"movie","i":{"imageUrl":"x"}});
        let s = json!({"l":"The Matrix","y":1999,"qid":"tvSeries","i":{"imageUrl":"x"}});
        let other = json!({"l":"The Matrix Reloaded","y":2003,"qid":"movie","i":{"imageUrl":"x"}});
        assert!(score(&m, "The Matrix", Some(1999), false) > score(&s, "The Matrix", Some(1999), false));
        assert!(score(&m, "The Matrix", Some(1999), false) > score(&other, "The Matrix", Some(1999), false));
        assert!(score(&m, "The Matrix", Some(1999), false) >= 4);
        assert!(score(&json!({"l":"x"}), "x", None, false) < 0);
        assert_eq!(resized("https://m.media-amazon.com/images/M/abc@._V1_.jpg"), "https://m.media-amazon.com/images/M/abc@._V1_QL80_UX400_.jpg");
        assert_eq!(encode("the matrix"), "the%20matrix");
    }
}

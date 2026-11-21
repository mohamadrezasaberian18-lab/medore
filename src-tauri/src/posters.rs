//! Online posters for movies and shows (on by default, switchable in
//! Settings › Library). Uses the public IMDb title-suggestion feed: no account
//! or key, and only the title is sent.

use std::time::Duration;

use serde_json::Value;

use crate::core::posters_match::{encode, resized, score};

pub struct Found {
    pub imdb: String,
    pub title: String,
    pub year: Option<i32>,
    pub image: Vec<u8>,
}

pub fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().provider(ureq::tls::TlsProvider::NativeTls).build();
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .user_agent("MEDORE/1.0")
        .tls_config(tls)
        .build()
        .into()
}

/// Err = the network is unavailable (try again later); Ok(None) = no match.
pub fn fetch(agent: &ureq::Agent, title: &str, year: Option<i32>, show: bool) -> Result<Option<Found>, String> {
    let q = title.trim().to_lowercase();
    if q.chars().filter(|c| c.is_alphanumeric()).count() < 2 {
        return Ok(None);
    }
    let url = format!("https://v3.sg.media-imdb.com/suggestion/x/{}.json", encode(&q));
    let body = agent.get(&url).call().map_err(|e| e.to_string())?.body_mut().read_to_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let cands = v["d"].as_array().cloned().unwrap_or_default();
    let best = cands.iter().map(|c| (score(c, title, year, show), c)).filter(|(s, _)| *s >= 4).max_by_key(|(s, _)| *s);
    let Some((_, c)) = best else { return Ok(None) };
    let img_url = resized(c["i"]["imageUrl"].as_str().unwrap_or_default());
    let image = agent.get(&img_url).call().map_err(|e| e.to_string())?.body_mut().read_to_vec().map_err(|e| e.to_string())?;
    if image.len() < 1000 {
        return Ok(None);
    }
    Ok(Some(Found {
        imdb: c["id"].as_str().unwrap_or_default().to_string(),
        title: c["l"].as_str().unwrap_or_default().to_string(),
        year: c["y"].as_i64().map(|y| y as i32),
        image,
    }))
}

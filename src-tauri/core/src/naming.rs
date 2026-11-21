//! Understand media file names: movies, series episodes, years, extras.
//!
//!   Breaking.Bad.S02E05.Breakage.1080p.BluRay.x264.mkv
//!       -> episode, show "Breaking Bad", season 2, episode 5, title "Breakage"
//!   The Matrix (1999) [2160p HDR].mkv -> movie "The Matrix", 1999
//!   Show Name/Season 3/07 - Some Title.mkv -> episode, season 3, episode 7

use std::path::{Component, Path};
use std::sync::OnceLock;

use fancy_regex::Regex;
use serde::Serialize;

const TAGS: &str = concat!(
    r"480p|540p|576p|720p|1080p|1080i|1440p|2160p|4320p|4k|8k|uhd|fhd|hd|sd|",
    r"bluray|blu ray|bdrip|brrip|bdremux|remux|webrip|web dl|webdl|web|hdtv|pdtv|dvdrip|dvdscr|dvd|hdrip|",
    r"x264|x265|h 264|h 265|h264|h265|hevc|avc|av1|vp9|xvid|divx|10bit|8bit|12bit|",
    r"hdr|hdr10|hdr10plus|hdr10\+|dv|dovi|dolby vision|sdr|",
    r"dts|dts hd|dtshd|truehd|atmos|aac|aac2|ac3|eac3|ddp|ddp5|dd5|flac|opus|mp3|5 1|7 1|2 0|",
    r"proper|repack|extended|unrated|uncut|imax|directors cut|remastered|",
    r"multi|dual|dubbed|subbed|internal|limited|complete|amzn|nf|dsnp|hmax|atvp|hulu|",
    r"farsi|persian|dubbed farsi|hardsub|softsub|zirnevis"
);

struct Rx {
    tag: Regex,
    ep: Vec<Regex>,
    ep_only: Regex,
    anime: Regex,
    leading_num: Regex,
    season_dir: Regex,
    year: Regex,
    brackets: Regex,
    extra: Regex,
    digit_dot: Regex,
    group_suffix: Regex,
}

fn rx() -> &'static Rx {
    static R: OnceLock<Rx> = OnceLock::new();
    R.get_or_init(|| Rx {
        tag: Regex::new(&format!(r"(?i)(?:^|\s)(?:{TAGS})(?=\s|$)")).unwrap(),
        ep: vec![
            Regex::new(r"(?i)(?:^|\s)s(\d{1,2})\s?e(\d{1,4})(?:\s?-?\s?e(\d{1,4}))?(?=\s|$|-)").unwrap(),
            Regex::new(r"(?i)(?:^|\s)(\d{1,2})x(\d{2,3})(?=\s|$)").unwrap(),
            Regex::new(r"(?i)(?:^|\s)season\s?(\d{1,2})\s?episode\s?(\d{1,4})(?=\s|$)").unwrap(),
        ],
        ep_only: Regex::new(r"(?i)(?:^|\s)(?:e|ep|episode|part|qesmat|ghesmat|قسمت)\s?(\d{1,4})(?=\s|$)").unwrap(),
        anime: Regex::new(r"\s-\s(\d{1,4})(?:v\d)?(?=\s|$)").unwrap(),
        leading_num: Regex::new(r"^(\d{1,3})(?:\s?[-.]\s?|\s|$)(.*)$").unwrap(),
        season_dir: Regex::new(r"(?i)^(?:season|series|s|fasl|فصل)\s?(\d{1,2})$").unwrap(),
        year: Regex::new(r"(?:^|\s)((?:19|20)\d{2})(?=\s|$)").unwrap(),
        brackets: Regex::new(r"\[[^\]]*\]|\{[^}]*\}").unwrap(),
        extra: Regex::new(r"(?i)(?:^|\s)(sample|trailer|featurette|behind the scenes|deleted scenes?|extras?)(?=\s|$)").unwrap(),
        digit_dot: Regex::new(r"(?<=\d)\.(?=\d)").unwrap(),
        group_suffix: Regex::new(r"-[A-Za-z0-9]{2,}$").unwrap(),
    })
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct ParsedName {
    pub title: String,
    pub kind: &'static str, // movie | episode | video
    pub year: Option<i32>,
    pub show: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub episode_end: Option<i32>,
    pub episode_title: Option<String>,
    pub extra: bool,
}

impl ParsedName {
    pub fn episode_tag(&self) -> Option<String> {
        let ep = self.episode?;
        let mut t = format!("S{:02}E{:02}", self.season.unwrap_or(1), ep);
        if let Some(e) = self.episode_end {
            if e != ep {
                t.push_str(&format!("-E{e:02}"));
            }
        }
        Some(t)
    }
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn normalize(stem: &str) -> String {
    let r = rx();
    let s = r.brackets.replace_all(stem, " ");
    let s: String = s.chars().map(|c| if c == '(' || c == ')' { ' ' } else { c }).collect();
    let s = r.digit_dot.replace_all(&s, " ");
    let s: String = s.chars().map(|c| if c == '.' || c == '_' { ' ' } else { c }).collect();
    collapse_ws(&s)
}

fn clean_title(s: &str) -> String {
    let s = collapse_ws(s);
    let s = s.trim_matches(|c: char| " -–—,:;~".contains(c)).to_string();
    if !s.is_empty() && s == s.to_lowercase() && s.chars().any(|c| c.is_ascii_alphabetic()) {
        return s
            .split(' ')
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    s
}

fn cut_at_tags(s: &str) -> String {
    match rx().tag.find(s).ok().flatten() {
        Some(m) => s[..m.start()].to_string(),
        None => s.to_string(),
    }
}

fn strip_group_suffix(s: &str) -> String {
    rx().group_suffix.replace(s, "").to_string()
}

fn last_year_before(s: &str) -> Option<(usize, i32)> {
    let mut found = None;
    for m in rx().year.captures_iter(s).flatten() {
        let g = m.get(1).unwrap();
        let whole = m.get(0).unwrap();
        if whole.start() > 0 || g.start() > 0 {
            found = Some((whole.start(), g.as_str().parse().unwrap_or(0)));
        }
    }
    found
}

fn stem_of(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Normalised parent directory names, nearest first.
fn parents(path: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(parent) = path.parent() {
        for c in parent.components() {
            match c {
                Component::Prefix(p) => out.push(p.as_os_str().to_string_lossy().trim_end_matches('\\').to_string()),
                Component::Normal(n) => out.push(normalize(&n.to_string_lossy())),
                _ => {}
            }
        }
    }
    out.reverse();
    out
}

pub fn parse(path: &Path) -> ParsedName {
    let r = rx();
    let raw = stem_of(path);
    let norm = normalize(&raw);
    let parents = parents(path);
    let extra = r.extra.is_match(&norm).unwrap_or(false);

    for re in &r.ep {
        if let Ok(Some(c)) = re.captures(&norm) {
            let whole = c.get(0).unwrap();
            let season: i32 = c.get(1).unwrap().as_str().parse().unwrap_or(1);
            let ep: i32 = c.get(2).unwrap().as_str().parse().unwrap_or(0);
            let ep_end = c.get(3).and_then(|m| m.as_str().parse().ok());
            let mut before = norm[..whole.start()].to_string();
            let after = &norm[whole.end()..];
            let mut show_year = None;
            if let Some((pos, y)) = last_year_before(&before) {
                if pos > 0 || before.trim_start() != before {
                    show_year = Some(y);
                    before = before[..pos].to_string();
                }
            }
            let mut show = clean_title(&cut_at_tags(&before));
            if show.is_empty() {
                show = show_from_dirs(&parents).unwrap_or_default();
            }
            let ep_title = clean_title(&strip_group_suffix(&cut_at_tags(after.trim_start_matches([' ', '-']))));
            return ParsedName {
                title: if show.is_empty() { norm.clone() } else { show.clone() },
                kind: "episode",
                year: show_year,
                show: (!show.is_empty()).then_some(show),
                season: Some(season),
                episode: Some(ep),
                episode_end: ep_end,
                episode_title: (!ep_title.is_empty()).then_some(ep_title),
                extra,
            };
        }
    }

    let season_from_dir = parents.first().and_then(|p| season_of_dir_norm(p));
    if let Some(season) = season_from_dir {
        let mut ep = None;
        let mut rest = norm.clone();
        if let Ok(Some(c)) = r.leading_num.captures(&norm) {
            ep = c.get(1).and_then(|m| m.as_str().parse().ok());
            rest = c.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
        } else if let Ok(Some(c)) = r.ep_only.captures(&norm) {
            ep = c.get(1).and_then(|m| m.as_str().parse().ok());
            rest = norm[c.get(0).unwrap().end()..].to_string();
        }
        if let Some(ep) = ep {
            let show = show_from_dirs(&parents);
            let t = clean_title(&cut_at_tags(&rest));
            return ParsedName {
                title: show.clone().unwrap_or_else(|| norm.clone()),
                kind: "episode",
                show,
                season: Some(season),
                episode: Some(ep),
                episode_title: (!t.is_empty()).then_some(t),
                extra,
                ..Default::default()
            };
        }
    }

    // "[Group] Show Name - 07 [1080p]" (anime style, implicit season 1)
    if let Ok(Some(c)) = r.anime.captures(&norm) {
        let whole = c.get(0).unwrap();
        if !r.year.is_match(whole.as_str()).unwrap_or(false) {
            let show = clean_title(&cut_at_tags(&norm[..whole.start()]));
            if !show.is_empty() {
                return ParsedName {
                    title: show.clone(),
                    kind: "episode",
                    show: Some(show),
                    season: Some(1),
                    episode: c.get(1).and_then(|m| m.as_str().parse().ok()),
                    extra,
                    ..Default::default()
                };
            }
        }
    }

    // movies
    if let Some((pos, y)) = last_year_before(&norm) {
        if pos > 0 {
            let title = clean_title(&cut_at_tags(&norm[..pos]));
            if !title.is_empty() {
                return ParsedName { title, kind: "movie", year: Some(y), extra, ..Default::default() };
            }
        }
    }

    let mut title = clean_title(&strip_group_suffix(&cut_at_tags(&norm)));
    if title.is_empty() {
        title = clean_title(&norm);
    }
    if title.is_empty() {
        title = raw.clone();
    }
    let kind = if r.tag.is_match(&norm).unwrap_or(false) { "movie" } else { "video" };
    ParsedName { title, kind, extra, ..Default::default() }
}

fn season_of_dir_norm(d: &str) -> Option<i32> {
    rx().season_dir.captures(d).ok().flatten().and_then(|c| c.get(1)).and_then(|m| m.as_str().parse().ok())
}

/// "Season 2", "S02", "Series 2", "فصل 2" -> 2
pub fn season_of_dir(name: &str) -> Option<i32> {
    season_of_dir_norm(&normalize(name))
}

fn show_from_dirs(parents: &[String]) -> Option<String> {
    const SKIP: &[&str] = &["tv", "tv shows", "series", "shows", "videos", "downloads", "media", "movies", "film", "films", "serial", "سریال"];
    for d in parents {
        if d.is_empty() || season_of_dir_norm(d).is_some() || SKIP.contains(&d.to_lowercase().as_str()) {
            continue;
        }
        let bytes = d.as_bytes();
        if (bytes.len() == 1 || (bytes.len() == 2 && bytes[1] == b':')) && bytes[0].is_ascii_alphabetic() {
            return None; // drive root
        }
        let mut d = d.clone();
        if let Some((pos, _)) = last_year_before(&d) {
            if pos > 0 {
                d.truncate(pos);
            }
        }
        let t = clean_title(&cut_at_tags(&d));
        return (!t.is_empty()).then_some(t);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(s: &str) -> ParsedName {
        parse(Path::new(s))
    }

    #[test]
    fn release_episode() {
        let n = p(r"D:\TV\Breaking.Bad.S02E05.Breakage.1080p.BluRay.x264-GRP.mkv");
        assert_eq!(n.kind, "episode");
        assert_eq!(n.show.as_deref(), Some("Breaking Bad"));
        assert_eq!((n.season, n.episode), (Some(2), Some(5)));
        assert_eq!(n.episode_title.as_deref(), Some("Breakage"));
    }

    #[test]
    fn movie_with_year() {
        let n = p(r"D:\Movies\The Matrix (1999) [2160p HDR].mkv");
        assert_eq!((n.kind, n.title.as_str(), n.year), ("movie", "The Matrix", Some(1999)));
        let n = p(r"D:\Movies\Dune.Part.Two.2024.1080p.WEB-DL.DDP5.1.Atmos.H.264-FLUX.mkv");
        assert_eq!((n.kind, n.title.as_str(), n.year), ("movie", "Dune Part Two", Some(2024)));
    }

    #[test]
    fn year_in_title() {
        let n = p(r"D:\Movies\Blade.Runner.2049.2017.1080p.mkv");
        assert_eq!((n.title.as_str(), n.year), ("Blade Runner 2049", Some(2017)));
    }

    #[test]
    fn season_folder() {
        let n = p(r"D:\TV\Show Name\Season 3\07 - Some Title.mkv");
        assert_eq!(n.kind, "episode");
        assert_eq!(n.show.as_deref(), Some("Show Name"));
        assert_eq!((n.season, n.episode), (Some(3), Some(7)));
        assert_eq!(n.episode_title.as_deref(), Some("Some Title"));
        let n = p(r"D:\TV\Show Name\Season 2\01.mkv");
        assert_eq!((n.season, n.episode), (Some(2), Some(1)));
    }

    #[test]
    fn other_episode_styles() {
        let n = p(r"D:\x\show.name.1x02.mkv");
        assert_eq!((n.season, n.episode), (Some(1), Some(2)));
        let n = p(r"D:\x\[Sub] Anime Show - 07 [1080p].mkv");
        assert_eq!((n.show.as_deref(), n.episode), (Some("Anime Show"), Some(7)));
        let n = p(r"D:\x\Show.S01E01E02.mkv");
        assert_eq!((n.episode, n.episode_end), (Some(1), Some(2)));
        assert_eq!(n.episode_tag().as_deref(), Some("S01E01-E02"));
    }

    #[test]
    fn extras_and_videos() {
        assert!(p(r"D:\Movies\Movie.2020.sample.mkv").extra);
        let n = p(r"C:\Users\x\Videos\birthday party.mp4");
        assert_eq!((n.kind, n.title.as_str()), ("video", "Birthday Party"));
    }

    #[test]
    fn season_dir_names() {
        assert_eq!(season_of_dir("Season 2"), Some(2));
        assert_eq!(season_of_dir("S02"), Some(2));
        assert_eq!(season_of_dir("فصل 3"), Some(3));
        assert_eq!(season_of_dir("Extras"), None);
    }
}

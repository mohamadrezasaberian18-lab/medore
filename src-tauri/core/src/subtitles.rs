//! Subtitle intelligence: discovery next to the video (even when names do not
//! match), subtitles inside .zip archives, language/flag detection, text
//! encoding detection (Persian Windows-1256 and friends), Persian letter
//! normalisation and the track-selection policy.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use encoding_rs::Encoding;
use serde::Serialize;
use serde_json::Value;

use super::formats::{is_subtitle, is_video, SUBTITLE_EXTS};
use super::languages;
use super::naming;

const SUB_DIR_NAMES: &[&str] = &["subs", "sub", "subtitles", "subtitle", "srt", "زیرنویس", "zirnevis"];
const FLAG_FORCED: &[&str] = &["forced", "force", "foreign"];
const FLAG_SDH: &[&str] = &["sdh", "hi", "cc", "hoh", "deaf"];
const AMBIGUOUS: &[&str] = &["may", "dan", "per", "ind", "fin", "nor", "tam", "ben", "pol", "geo", "arm", "hun", "slo", "dut"];
pub const TEXT_SUB_EXTS: &[&str] = &["srt", "ass", "ssa", "vtt", "webvtt", "smi", "sami", "lrc"];
pub const RTL_LANGS: &[&str] = &["fa", "ar", "ur", "he", "ps", "ku"];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Candidate {
    pub path: String,
    pub lang: Option<String>,
    pub forced: bool,
    pub sdh: bool,
    pub score: i32,
    pub label: String,
}

// ---------------------------------------------------------------------------
// Language and flags from a file name

fn tokens(s: &str) -> Vec<String> {
    s.split(|c: char| c.is_whitespace() || "._-[]()".contains(c))
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

pub fn tags_from_name(sub_stem: &str, video_stem: Option<&str>) -> (Option<String>, bool, bool) {
    let mut rest = sub_stem;
    if let Some(v) = video_stem {
        if sub_stem.to_lowercase().starts_with(&v.to_lowercase()) && sub_stem.is_char_boundary(v.len()) {
            rest = &sub_stem[v.len()..];
        }
    }
    let toks = tokens(rest);
    let mut lang: Option<String> = None;
    let (mut forced, mut sdh) = (false, false);
    // The language is usually the last meaningful token: scan from the end.
    // Two-letter codes and ambiguous words only count in the very last place
    // so "It Follows" is not mistaken for Italian.
    let mut pos = 0;
    for t in toks.iter().rev() {
        if FLAG_FORCED.contains(&t.as_str()) {
            forced = true;
            continue;
        }
        if FLAG_SDH.contains(&t.as_str()) {
            sdh = true;
            continue;
        }
        if lang.is_none() && pos < 3 {
            if let Some(n) = languages::normalize(Some(t)) {
                if pos == 0 || (t.chars().count() >= 3 && !AMBIGUOUS.contains(&t.as_str())) {
                    lang = Some(n.to_string());
                }
            }
        }
        pos += 1;
    }
    // YTS style "2_English.srt"
    if lang.is_none() {
        if let Some((num, name)) = sub_stem.split_once('_') {
            if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                lang = languages::normalize(Some(name.split('_').next().unwrap_or(""))).map(str::to_string);
            }
        }
    }
    (lang, forced, sdh)
}

// ---------------------------------------------------------------------------
// Encoding detection

fn hint_encoding(lang: Option<&str>) -> Option<&'static str> {
    Some(match languages::normalize(lang)? {
        "fa" | "ar" | "ur" => "windows-1256",
        "ru" | "uk" | "bg" => "windows-1251",
        "el" => "windows-1253",
        "he" => "windows-1255",
        "tr" => "windows-1254",
        "pl" | "cs" | "hu" => "windows-1250",
        "th" => "windows-874",
        "zh" => "gb18030",
        "ja" => "shift_jis",
        "ko" => "euc-kr",
        _ => return None,
    })
}

/// (encoding label, frequent letters, expected share of all letters)
const PROFILES: &[(&str, &str, f64)] = &[
    ("windows-1256", "اینیردمهوتبسکلزشكيگ", 0.78), // Persian
    ("windows-1256", "الیيمونهربتعفةكدسق", 0.80),  // Arabic
    ("windows-1251", "оеаинтсрвлкмдпуяыь", 0.85),
    ("windows-1251", "оаниветірсклудмпяз", 0.80),
    ("windows-1253", "αοιετσνηυρπκμλςάέίόή", 0.85),
    ("windows-1255", "יוהלארתבמשנעדכם", 0.78),
    ("windows-874", "านรอกเงมยลวดทสตะ", 0.62),
    ("gb18030", "的一是不了人我在有他这中大来上国个到说们为子和你地出道也时年要就那", 0.30),
    ("big5", "的一是不了人我在有他這中大來上國個到說們為子和你地出道也時年要就那", 0.30),
    ("shift_jis", "のにはをたがでてとしいれなるかもうっすこ", 0.45),
    ("euc-kr", "이다는의에가을하고지서를기한로요", 0.40),
];
const LATIN_SETS: &[(&str, &str)] = &[
    ("windows-1252", "éèêëàâäçîïôöùûüÿñáíóúãõßœæøåÉÈÀÇÖÜÑ"),
    ("windows-1250", "ąćęłńóśźżčďěňřšťůýžőűăâîÁČĎÉĚÍŇÓŘŠŤÚŮÝŽŁŚŻ"),
    ("windows-1254", "çğıöşüâîÇĞİÖŞÜ"),
];

/// True when non-ASCII bytes sit inside ASCII words (é in café), as in French
/// or Polish, rather than forming whole words (Persian, Greek).
fn latin_like(sample: &[u8]) -> bool {
    let mut words: Vec<&[u8]> = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &b) in sample.iter().enumerate() {
        let is_word = b.is_ascii_alphabetic() || b >= 0x80;
        match (is_word, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                words.push(&sample[s..i]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        words.push(&sample[s..]);
    }
    let hi: Vec<&&[u8]> = words.iter().filter(|w| w.iter().any(|&b| b >= 0x80)).collect();
    if hi.is_empty() {
        return true;
    }
    let mixed = hi.iter().filter(|w| w.iter().any(|b| b.is_ascii_alphabetic())).count();
    mixed as f64 / hi.len() as f64 > 0.5
}

fn is_bad_char(c: char) -> bool {
    let o = c as u32;
    c.is_control() || (0xE000..=0xF8FF).contains(&o) || o == 0xFFFD
}

fn coverage(text: &str, letters: &str) -> f64 {
    let high: Vec<char> = text.chars().filter(|c| (*c as u32) > 127 && c.is_alphabetic()).collect();
    if high.is_empty() {
        return 0.0;
    }
    let bad = text.chars().filter(|c| (*c as u32) > 127 && is_bad_char(*c)).count();
    let hits = high.iter().filter(|c| letters.contains(**c)).count();
    hits as f64 / high.len() as f64 - bad as f64 / high.len().max(1) as f64
}

fn decode_strict(enc: &'static Encoding, data: &[u8]) -> Option<String> {
    enc.decode_without_bom_handling_and_without_replacement(data).map(|c| c.into_owned())
}

fn enc(label: &str) -> &'static Encoding {
    Encoding::for_label(label.as_bytes()).unwrap_or(encoding_rs::WINDOWS_1252)
}

/// Best guess for the text encoding of a subtitle file (an encoding_rs label).
pub fn detect_encoding(data: &[u8], hint_lang: Option<&str>) -> &'static str {
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return "utf-8";
    }
    if data.starts_with(&[0xFF, 0xFE]) {
        return "utf-16le";
    }
    if data.starts_with(&[0xFE, 0xFF]) {
        return "utf-16be";
    }
    let sample = &data[..data.len().min(200_000)];
    if sample.len() > 64 {
        let odd_zero = sample.iter().skip(1).step_by(2).filter(|&&b| b == 0).count();
        let even_zero = sample.iter().step_by(2).filter(|&&b| b == 0).count();
        if odd_zero as f64 > sample.len() as f64 * 0.3 {
            return "utf-16le";
        }
        if even_zero as f64 > sample.len() as f64 * 0.3 {
            return "utf-16be";
        }
    }
    match std::str::from_utf8(sample) {
        Ok(_) => return "utf-8",
        Err(e) => {
            // a cut at the very end of the sample is not a real error
            if e.error_len().is_none() && data.len() > sample.len() {
                return "utf-8";
            }
        }
    }
    let hinted = hint_encoding(hint_lang);
    let mut best = "windows-1252";
    let mut best_score = -9.0;
    if latin_like(sample) {
        for (label, letters) in LATIN_SETS {
            if let Some(text) = decode_strict(enc(label), sample) {
                let s = coverage(&text, letters) + if Some(*label) == hinted { 0.2 } else { 0.0 };
                if s > best_score + 1e-9 {
                    best = label;
                    best_score = s;
                }
            }
        }
        return best;
    }
    for (label, letters, expected) in PROFILES {
        if let Some(text) = decode_strict(enc(label), sample) {
            let s = coverage(&text, letters) / expected + if Some(*label) == hinted { 0.2 } else { 0.0 };
            if s > best_score + 1e-9 {
                best = label;
                best_score = s;
            }
        }
    }
    best
}

pub fn decode(data: &[u8], label: &str) -> String {
    let e = enc(label);
    let body = if label == "utf-8" && data.starts_with(&[0xEF, 0xBB, 0xBF]) { &data[3..] } else { data };
    let (text, _, _) = e.decode(body);
    text.into_owned()
}

pub fn read_text(path: &Path, hint_lang: Option<&str>) -> Option<(String, &'static str)> {
    let data = fs::read(path).ok()?;
    let e = detect_encoding(&data, hint_lang);
    Some((decode(&data, e), e))
}

/// Arabic yeh/kaf -> Persian. Windows-1256 Persian subtitles use the Arabic
/// letters, whose final forms look wrong in Persian.
pub fn normalize_persian(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'ي' | 'ى' => 'ی',
            'ك' => 'ک',
            _ => c,
        })
        .collect()
}

/// Make every ASS style use automatic (Unicode bidi) direction: Encoding -1.
pub fn ass_auto_direction(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 64);
    let mut enc_index: Option<usize> = None;
    let mut section = String::new();
    for line in text.split_inclusive('\n') {
        let s = line.trim();
        let mut line_out = line.to_string();
        if s.starts_with('[') && s.ends_with(']') {
            section = s.to_lowercase();
            enc_index = None;
        } else if (section == "[v4+ styles]" || section == "[v4 styles]") && s.to_lowercase().starts_with("format:") {
            let fields: Vec<String> = s[7..].split(',').map(|f| f.trim().to_lowercase()).collect();
            enc_index = fields.iter().position(|f| f == "encoding");
        } else if (section == "[v4+ styles]" || section == "[v4 styles]") && s.to_lowercase().starts_with("style:") {
            if let (Some(idx), Some((head, body))) = (enc_index, line.split_once(':')) {
                let mut parts: Vec<String> = body.split(',').map(str::to_string).collect();
                if idx < parts.len() {
                    let old = parts[idx].clone();
                    let ending = if old.ends_with("\r\n") {
                        "\r\n"
                    } else if old.ends_with('\n') {
                        "\n"
                    } else {
                        ""
                    };
                    parts[idx] = format!(" -1{ending}");
                    line_out = format!("{head}:{}", parts.join(","));
                }
            }
        }
        out.push_str(&line_out);
    }
    out
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Return (path to give the player, encoding hint). Persian text subtitles
/// are rewritten once as normalised UTF-8 into the cache; RTL ASS files get
/// automatic direction when asked; non-UTF-8 files are converted to UTF-8 so
/// libass never guesses. The user's file is never modified.
pub fn prepare_for_playback(path: &Path, lang: Option<&str>, cache: &Path, persian_fix: bool, rtl_ass: bool) -> String {
    let ext = super::formats::ext_of(path);
    if !TEXT_SUB_EXTS.contains(&ext.as_str()) {
        return path.to_string_lossy().into_owned();
    }
    let Ok(data) = fs::read(path) else { return path.to_string_lossy().into_owned() };
    if data.len() > 20_000_000 {
        return path.to_string_lossy().into_owned();
    }
    let e = detect_encoding(&data, lang);
    let text = decode(&data, e);
    let norm = languages::normalize(lang).or_else(|| guess_language_from_text(&text).and_then(|l| languages::normalize(Some(l))));
    let want_persian = persian_fix && norm == Some("fa");
    let want_rtl = rtl_ass && (ext == "ass" || ext == "ssa") && norm.map(|n| RTL_LANGS.contains(&n)).unwrap_or(false);
    let mut fixed = if want_persian { normalize_persian(&text) } else { text.clone() };
    if want_rtl {
        fixed = ass_auto_direction(&fixed);
    }
    if fixed == text && e == "utf-8" {
        return path.to_string_lossy().into_owned();
    }
    let meta = fs::metadata(path).ok();
    let key = format!(
        "{}|{}|{:?}|{want_persian}|{want_rtl}",
        path.display(),
        meta.as_ref().map(|m| m.len()).unwrap_or(0),
        meta.and_then(|m| m.modified().ok())
    );
    let out = cache.join(format!("{:016x}.{ext}", fnv(&key)));
    if !out.exists() {
        let _ = fs::create_dir_all(cache);
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(fixed.as_bytes());
        if fs::write(&out, bytes).is_err() {
            return path.to_string_lossy().into_owned();
        }
    }
    out.to_string_lossy().into_owned()
}

fn in_ranges(c: char, ranges: &[(u32, u32)]) -> bool {
    let o = c as u32;
    ranges.iter().any(|(lo, hi)| (*lo..=*hi).contains(&o))
}
const ARABIC: &[(u32, u32)] = &[(0x0600, 0x06FF), (0x0750, 0x077F), (0xFB50, 0xFDFF), (0xFE70, 0xFEFF)];

/// Tell Persian from Arabic (and spot a few other scripts) by content.
pub fn guess_language_from_text(text: &str) -> Option<&'static str> {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() < 20 {
        return None;
    }
    let n = letters.len() as f64;
    let arabic = letters.iter().filter(|c| in_ranges(**c, ARABIC)).count() as f64;
    if arabic > n * 0.3 {
        let persian_only = text.chars().filter(|c| "پچژگکی".contains(*c)).count();
        let arabic_only = text.chars().filter(|c| "ةيكىإأ".contains(*c)).count();
        return Some(if persian_only >= arabic_only { "fa" } else { "ar" });
    }
    let count = |r: &[(u32, u32)]| letters.iter().filter(|c| in_ranges(**c, r)).count() as f64;
    if count(&[(0x0400, 0x04FF)]) > n * 0.5 {
        return Some("ru");
    }
    if count(&[(0x0590, 0x05FF)]) > n * 0.5 {
        return Some("he");
    }
    if count(&[(0x0370, 0x03FF)]) > n * 0.5 {
        return Some("el");
    }
    if letters.iter().filter(|c| c.is_ascii_alphabetic()).count() as f64 > n * 0.8 {
        return Some("en");
    }
    None
}

pub fn sniff_language(path: &Path) -> Option<&'static str> {
    let ext = super::formats::ext_of(path);
    if !["srt", "ass", "ssa", "vtt", "webvtt", "smi", "sami", "sub", "txt"].contains(&ext.as_str()) {
        return None;
    }
    if fs::metadata(path).ok()?.len() > 8_000_000 {
        return None;
    }
    let (text, _) = read_text(path, None)?;
    // keep dialogue only: drop timing lines and ASS override tags
    let mut clean = String::with_capacity(text.len().min(60_000));
    let mut in_brace = false;
    for line in text.lines().take(4000) {
        if line.contains("-->") || line.trim().chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let l = line.strip_prefix("Dialogue:").map(|r| r.splitn(10, ',').nth(9).unwrap_or("")).unwrap_or(line);
        for c in l.chars() {
            match c {
                '{' => in_brace = true,
                '}' => in_brace = false,
                _ if !in_brace => clean.push(c),
                _ => {}
            }
        }
        clean.push(' ');
        if clean.len() > 50_000 {
            break;
        }
    }
    guess_language_from_text(&clean)
}

// ---------------------------------------------------------------------------
// Subtitles inside .zip archives (as most subtitle sites deliver them)

/// Extract subtitle files from .zip archives in `dir` into the cache and
/// return their paths. Archives are extracted once (keyed by size + mtime).
pub fn subtitles_from_zips(dir: &Path, cache: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if super::formats::ext_of(&p) != "zip" {
            continue;
        }
        let Ok(meta) = e.metadata() else { continue };
        if meta.len() > 30_000_000 {
            continue;
        }
        let key = format!("{}|{}|{:?}", p.display(), meta.len(), meta.modified().ok());
        let target = cache.join("zip").join(format!("{:016x}", fnv(&key)));
        let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if !target.exists() {
            let Ok(f) = fs::File::open(&p) else { continue };
            let Ok(mut z) = zip::ZipArchive::new(f) else { continue };
            let mut wrote = false;
            for i in 0..z.len().min(200) {
                let Ok(mut file) = z.by_index(i) else { continue };
                if !file.is_file() || file.size() > 20_000_000 {
                    continue;
                }
                let name = file.enclosed_name().and_then(|n| n.file_name().map(|s| s.to_owned()));
                let Some(name) = name else { continue };
                if !is_subtitle(Path::new(&name)) {
                    continue;
                }
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_err() {
                    continue;
                }
                let _ = fs::create_dir_all(&target);
                if fs::write(target.join(&name), buf).is_ok() {
                    wrote = true;
                }
            }
            if !wrote {
                let _ = fs::create_dir_all(&target); // remember: nothing inside
            }
        }
        if let Ok(rd2) = fs::read_dir(&target) {
            for f in rd2.flatten() {
                out.push(f.path());
            }
        }
        let _ = stem;
    }
    out
}

// ---------------------------------------------------------------------------
// Discovery

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn is_sample_or_extra(p: &Path) -> bool {
    naming::parse(p).extra
}

/// Find subtitle files that belong to `video`, best first.
///
/// Matching, strongest first: same file name start (100), same name after
/// normalising separators (90), same parsed title + year (80), same episode
/// (S01E02) (70), Subs/<video name>/ folder (60), and when the folder holds a
/// single main video, every subtitle there (40).
pub fn find_external(video: &Path, cache: &Path, sniff: bool) -> Vec<Candidate> {
    let Some(dir) = video.parent() else { return vec![] };
    let Ok(rd) = fs::read_dir(dir) else { return vec![] };
    let entries: Vec<fs::DirEntry> = rd.flatten().collect();
    let vstem = stem(video);
    let vparsed = naming::parse(video);
    let vtag = vparsed.episode_tag();
    let vnorm = naming::normalize(&vstem).to_lowercase();
    let vtitle = vparsed.title.to_lowercase();

    let main_videos = entries
        .iter()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| is_video(p) && !is_sample_or_extra(p))
        .count();
    let only_video = main_videos <= 1;

    let mut found: HashMap<String, Candidate> = HashMap::new();
    let mut consider = |path: &Path, base: i32, from_zip: bool| {
        let ext = super::formats::ext_of(path);
        if !SUBTITLE_EXTS.contains(&ext.as_str()) {
            return;
        }
        if ext == "sub" && path.with_extension("idx").exists() {
            return; // VobSub: the .idx references the .sub
        }
        let sstem = stem(path);
        let low = sstem.to_lowercase();
        let mut score = 0;
        if low.starts_with(&vstem.to_lowercase()) {
            score = 100;
        } else if naming::normalize(&sstem).to_lowercase().starts_with(&vnorm) {
            score = 90;
        } else {
            let sp = naming::parse(path);
            if let Some(vt) = &vtag {
                if sp.episode_tag().as_ref() == Some(vt) {
                    score = 70;
                }
            } else if !vtitle.is_empty()
                && sp.title.to_lowercase() == vtitle
                && (vparsed.year.is_none() || sp.year.is_none() || sp.year == vparsed.year)
            {
                score = 80;
            }
        }
        if score == 0 && base >= 60 {
            score = base;
        }
        if score == 0 && only_video {
            score = if from_zip { 35 } else { 40 };
        }
        if score == 0 {
            return;
        }
        let video_prefix = if low.starts_with(&vstem.to_lowercase()) { Some(vstem.as_str()) } else { None };
        let (mut lang, forced, sdh) = tags_from_name(&sstem, video_prefix);
        if lang.is_none() && sniff {
            lang = sniff_language(path).map(str::to_string);
        }
        let key = path.to_string_lossy().into_owned();
        let c = Candidate {
            path: key.clone(),
            lang,
            forced,
            sdh,
            score,
            label: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        };
        match found.get(&key) {
            Some(prev) if prev.score >= c.score => {}
            _ => {
                found.insert(key, c);
            }
        }
    };

    for e in &entries {
        if e.file_type().map(|t| t.is_file()).unwrap_or(false) {
            consider(&e.path(), 0, false);
        }
    }
    for e in &entries {
        if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_lowercase();
        if !SUB_DIR_NAMES.contains(&name.as_str()) {
            continue;
        }
        let Ok(sub) = fs::read_dir(e.path()) else { continue };
        for se in sub.flatten() {
            let ft = se.file_type();
            if ft.as_ref().map(|t| t.is_file()).unwrap_or(false) {
                consider(&se.path(), 0, false);
            } else if ft.map(|t| t.is_dir()).unwrap_or(false) && naming::normalize(&se.file_name().to_string_lossy()).to_lowercase() == vnorm {
                if let Ok(inner) = fs::read_dir(se.path()) {
                    for f in inner.flatten() {
                        consider(&f.path(), 60, false);
                    }
                }
            }
        }
    }
    for p in subtitles_from_zips(dir, cache) {
        consider(&p, 0, true);
    }
    let mut v: Vec<Candidate> = found.into_values().collect();
    v.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.to_lowercase().cmp(&b.path.to_lowercase())));
    v
}

// ---------------------------------------------------------------------------
// Track selection policy

#[derive(Debug, Default, PartialEq)]
pub struct TrackChoice {
    pub aid: Option<i64>,
    pub sid: Option<i64>,
}

fn s<'a>(t: &'a Value, k: &str) -> Option<&'a str> {
    t.get(k).and_then(|v| v.as_str())
}

fn is_forced(t: &Value) -> bool {
    t["forced"].as_bool().unwrap_or(false) || s(t, "title").map(|x| x.to_lowercase().contains("forced")).unwrap_or(false)
}

fn is_sdh(t: &Value) -> bool {
    t["hearing-impaired"].as_bool().unwrap_or(false)
        || s(t, "title").map(|x| {
            let l = x.to_lowercase();
            l.contains("sdh") || l.contains("hearing") || l.contains(" cc")
        })
        .unwrap_or(false)
}

/// Pick the audio and subtitle tracks a person would most likely want.
/// `sub_prefs` lists languages the viewer reads, best first. When the spoken
/// language is one of them only forced subtitles are shown.
pub fn choose_tracks(tracks: &[Value], audio_prefs: &[String], sub_prefs: &[String], subs_enabled: bool) -> TrackChoice {
    let mut out = TrackChoice::default();
    let audio: Vec<&Value> = tracks.iter().filter(|t| s(t, "type") == Some("audio")).collect();
    let subs: Vec<&Value> = tracks.iter().filter(|t| s(t, "type") == Some("sub")).collect();

    let mut chosen_audio: Option<&Value> = None;
    'outer: for pref in audio_prefs {
        for t in &audio {
            if languages::same(s(t, "lang"), Some(pref)) {
                chosen_audio = Some(t);
                break 'outer;
            }
        }
    }
    if chosen_audio.is_none() && !audio.is_empty() {
        chosen_audio = audio.iter().find(|t| t["default"].as_bool().unwrap_or(false)).copied().or(Some(audio[0]));
    }
    out.aid = chosen_audio.and_then(|t| t["id"].as_i64());
    let audio_lang = chosen_audio.and_then(|t| languages::normalize(s(t, "lang")));

    if !subs_enabled || subs.is_empty() {
        return out;
    }
    let rank = |t: &Value| {
        let codec = s(t, "codec").unwrap_or("");
        let image = ["hdmv_pgs_subtitle", "dvd_subtitle", "pgssub", "dvdsub", "dvb_subtitle"].contains(&codec);
        (is_forced(t), is_sdh(t), image, !t["external"].as_bool().unwrap_or(false), !t["default"].as_bool().unwrap_or(false), t["id"].as_i64().unwrap_or(0))
    };
    // Only the first (native) subtitle language counts as "understood audio":
    // a Persian viewer listing Persian then English still wants subtitles on
    // an English film — Persian ones, or English ones when there are none.
    let native = sub_prefs.first().and_then(|p| languages::normalize(Some(p)));
    if let Some(al) = audio_lang {
        if native == Some(al) {
            if let Some(f) = subs.iter().find(|t| is_forced(t) && languages::same(s(t, "lang"), Some(al))) {
                out.sid = f["id"].as_i64();
            }
            return out;
        }
    }
    for pref in sub_prefs {
        let mut c: Vec<&&Value> = subs.iter().filter(|t| languages::same(s(t, "lang"), Some(pref)) && !is_forced(t)).collect();
        if !c.is_empty() {
            c.sort_by_key(|t| rank(t));
            out.sid = c[0]["id"].as_i64();
            return out;
        }
    }
    if let Some(f) = subs.iter().find(|t| is_forced(t) && (audio_lang.is_none() || languages::same(s(t, "lang"), audio_lang))) {
        out.sid = f["id"].as_i64();
        return out;
    }
    // Many releases (e.g. Persian "SoftSub" files) carry their subtitle with
    // no language tag, marked default and/or forced: that is the one meant
    // to be shown.
    let untagged = |t: &&&Value| languages::normalize(s(t, "lang")).is_none();
    if let Some(d) = subs.iter().filter(untagged).find(|t| t["default"].as_bool().unwrap_or(false) || is_forced(t)) {
        out.sid = d["id"].as_i64();
        return out;
    }
    if subs.len() == 1 && languages::normalize(s(subs[0], "lang")).is_none() {
        out.sid = subs[0]["id"].as_i64();
        return out;
    }
    // Nothing in a preferred language: an external subtitle the person put
    // next to the video is still almost certainly wanted.
    if let Some(e) = subs.iter().find(|t| t["external"].as_bool().unwrap_or(false)) {
        out.sid = e["id"].as_i64();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tags() {
        assert_eq!(tags_from_name("Movie.2020.fa", Some("Movie.2020")), (Some("fa".into()), false, false));
        assert_eq!(tags_from_name("Movie.English.forced", None), (Some("en".into()), true, false));
        assert_eq!(tags_from_name("It.Follows.2014", None).0, None);
        assert_eq!(tags_from_name("2_English", None).0, Some("en".into()));
        assert_eq!(tags_from_name("Movie.per.sdh", None), (Some("fa".into()), false, true));
    }

    #[test]
    fn persian_cp1256_detected() {
        // "سلام دنیا این یک زیرنویس فارسی است" in Windows-1256 (Arabic yeh)
        let text = "1\r\n00:00:01,000 --> 00:00:02,000\r\nسلام دنيا اين يك زيرنويس فارسي است و كار مي كند\r\n";
        let (bytes, _, _) = encoding_rs::WINDOWS_1256.encode(text);
        assert_eq!(detect_encoding(&bytes, None), "windows-1256");
        let back = decode(&bytes, "windows-1256");
        assert!(back.contains("زيرنويس"));
        assert_eq!(guess_language_from_text(&normalize_persian(&back)), Some("fa"));
    }

    #[test]
    fn russian_and_french() {
        let ru = "Привет, как дела? Это тестовый субтитр на русском языке, и он должен читаться правильно.";
        let (b, _, _) = encoding_rs::WINDOWS_1251.encode(ru);
        assert_eq!(detect_encoding(&b, None), "windows-1251");
        let fr = "Où est la bibliothèque? Le garçon a mangé une crêpe très délicieuse à la fête.";
        let (b, _, _) = encoding_rs::WINDOWS_1252.encode(fr);
        assert_eq!(detect_encoding(&b, None), "windows-1252");
        assert_eq!(detect_encoding("plain ascii".as_bytes(), None), "utf-8");
        assert_eq!(detect_encoding("زیرنویس".as_bytes(), None), "utf-8");
    }

    #[test]
    fn persian_letters() {
        assert_eq!(normalize_persian("كيك"), "کیک");
    }

    #[test]
    fn ass_direction() {
        let src = "[V4+ Styles]\nFormat: Name, Fontname, Encoding\nStyle: Default,Arial,1\n[Events]\n";
        let out = ass_auto_direction(src);
        assert!(out.contains("Style: Default,Arial, -1\n"), "{out}");
    }

    #[test]
    fn choose_persian_subs_for_english_audio() {
        let tracks = vec![
            json!({"id":1,"type":"audio","lang":"eng","default":true}),
            json!({"id":1,"type":"sub","lang":"eng"}),
            json!({"id":2,"type":"sub","lang":"per"}),
        ];
        let c = choose_tracks(&tracks, &[], &["fa".into(), "en".into()], true);
        assert_eq!(c, TrackChoice { aid: Some(1), sid: Some(2) });
    }

    #[test]
    fn native_audio_shows_no_subs_and_fallback_to_english() {
        let fa_audio = vec![json!({"id":1,"type":"audio","lang":"per"}), json!({"id":4,"type":"sub","lang":"eng"})];
        assert_eq!(choose_tracks(&fa_audio, &[], &["fa".into(), "en".into()], true).sid, None);
        let en_only = vec![json!({"id":1,"type":"audio","lang":"eng"}), json!({"id":5,"type":"sub","lang":"eng"})];
        assert_eq!(choose_tracks(&en_only, &[], &["fa".into(), "en".into()], true).sid, Some(5));
    }

    #[test]
    fn untagged_default_forced_softsub_selected() {
        let t = vec![
            json!({"id":1,"type":"sub","codec":"subrip","default":true,"forced":true}),
            json!({"id":1,"type":"audio","lang":"eng","default":true}),
        ];
        assert_eq!(choose_tracks(&t, &[], &["fa".into(), "en".into()], true).sid, Some(1));
    }

    #[test]
    fn external_unknown_language_still_selected() {
        let tracks = vec![json!({"id":1,"type":"audio","lang":"eng"}), json!({"id":3,"type":"sub","external":true})];
        let c = choose_tracks(&tracks, &[], &["fa".into()], true);
        assert_eq!(c.sid, Some(3));
    }

    #[test]
    fn discovery_with_different_names() {
        let dir = std::env::temp_dir().join(format!("vesper-subs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Subs")).unwrap();
        fs::write(dir.join("Dune.Part.Two.2024.1080p.WEB-DL.x264.mkv"), b"x").unwrap();
        fs::write(dir.join("Dune.Part.Two.2024.sample.mkv"), b"x").unwrap();
        fs::write(dir.join("Dune Part Two 2024 BluRay.srt"), "1\n00:00:01,000 --> 00:00:02,000\nHello there friend how are you doing today\n").unwrap();
        fs::write(dir.join("Subs").join("Farsi.srt"), "1\n00:00:01,000 --> 00:00:02,000\nسلام\n").unwrap();
        // a zip with a subtitle inside
        {
            let f = fs::File::create(dir.join("subtitle-pack.zip")).unwrap();
            let mut z = zip::ZipWriter::new(f);
            z.start_file("Dune.Part.Two.Persian.srt", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, "1\n00:00:01,000 --> 00:00:02,000\nسلام دنیا\n".as_bytes()).unwrap();
            z.finish().unwrap();
        }
        let cache = dir.join("cache");
        let found = find_external(&dir.join("Dune.Part.Two.2024.1080p.WEB-DL.x264.mkv"), &cache, true);
        let names: Vec<&str> = found.iter().map(|c| c.label.as_str()).collect();
        assert!(names.contains(&"Dune Part Two 2024 BluRay.srt"), "{names:?}");
        assert!(names.contains(&"Farsi.srt"), "{names:?}");
        assert!(names.contains(&"Dune.Part.Two.Persian.srt"), "{names:?}");
        let fa = found.iter().find(|c| c.label == "Farsi.srt").unwrap();
        assert_eq!(fa.lang.as_deref(), Some("fa"));
        let _ = fs::remove_dir_all(&dir);
    }
}

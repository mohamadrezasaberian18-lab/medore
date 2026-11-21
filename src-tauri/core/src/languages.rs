//! Language codes: normalise ISO 639-1/639-2 codes and common names.

use std::collections::HashMap;
use std::sync::OnceLock;

/// canonical ISO 639-1 code, English name, native name, aliases
pub const LANGS: &[(&str, &str, &str, &[&str])] = &[
    ("en", "English", "English", &["eng", "english", "enus", "en-us", "en-gb", "engb"]),
    ("fa", "Persian", "فارسی", &["fas", "per", "persian", "farsi", "pes", "prs", "fa-ir", "parsi"]),
    ("ar", "Arabic", "العربية", &["ara", "arabic", "arb"]),
    ("fr", "French", "Français", &["fre", "fra", "french", "fr-fr", "fr-ca"]),
    ("de", "German", "Deutsch", &["ger", "deu", "german"]),
    ("es", "Spanish", "Español", &["spa", "spanish", "es-es", "es-419", "esla", "castilian", "latino"]),
    ("it", "Italian", "Italiano", &["ita", "italian"]),
    ("pt", "Portuguese", "Português", &["por", "portuguese", "pt-br", "ptbr", "pob", "brazilian", "pt-pt"]),
    ("ru", "Russian", "Русский", &["rus", "russian"]),
    ("tr", "Turkish", "Türkçe", &["tur", "turkish"]),
    ("zh", "Chinese", "中文", &["chi", "zho", "chinese", "chs", "cht", "zh-cn", "zh-tw", "zh-hans", "zh-hant", "cmn", "yue"]),
    ("ja", "Japanese", "日本語", &["jpn", "japanese", "jp"]),
    ("ko", "Korean", "한국어", &["kor", "korean"]),
    ("hi", "Hindi", "हिन्दी", &["hin", "hindi"]),
    ("ur", "Urdu", "اردو", &["urd", "urdu"]),
    ("he", "Hebrew", "עברית", &["heb", "hebrew", "iw"]),
    ("nl", "Dutch", "Nederlands", &["dut", "nld", "dutch", "flemish"]),
    ("sv", "Swedish", "Svenska", &["swe", "swedish"]),
    ("no", "Norwegian", "Norsk", &["nor", "norwegian", "nob", "nb", "nno", "nn"]),
    ("da", "Danish", "Dansk", &["dan", "danish"]),
    ("fi", "Finnish", "Suomi", &["fin", "finnish"]),
    ("pl", "Polish", "Polski", &["pol", "polish"]),
    ("cs", "Czech", "Čeština", &["cze", "ces", "czech"]),
    ("el", "Greek", "Ελληνικά", &["gre", "ell", "greek"]),
    ("hu", "Hungarian", "Magyar", &["hun", "hungarian"]),
    ("ro", "Romanian", "Română", &["rum", "ron", "romanian"]),
    ("id", "Indonesian", "Bahasa Indonesia", &["ind", "indonesian"]),
    ("ms", "Malay", "Bahasa Melayu", &["may", "msa", "malay"]),
    ("th", "Thai", "ไทย", &["tha", "thai"]),
    ("vi", "Vietnamese", "Tiếng Việt", &["vie", "vietnamese"]),
    ("uk", "Ukrainian", "Українська", &["ukr", "ukrainian"]),
    ("ku", "Kurdish", "Kurdî", &["kur", "kurdish", "ckb", "kmr"]),
    ("az", "Azerbaijani", "Azərbaycan", &["aze", "azerbaijani"]),
    ("hy", "Armenian", "Հայերեն", &["arm", "hye", "armenian"]),
    ("ka", "Georgian", "ქართული", &["geo", "kat", "georgian"]),
    ("bg", "Bulgarian", "Български", &["bul", "bulgarian"]),
    ("hr", "Croatian", "Hrvatski", &["hrv", "croatian"]),
    ("sr", "Serbian", "Српски", &["srp", "serbian"]),
    ("sk", "Slovak", "Slovenčina", &["slo", "slk", "slovak"]),
    ("sl", "Slovenian", "Slovenščina", &["slv", "slovenian"]),
    ("bn", "Bengali", "বাংলা", &["ben", "bengali"]),
    ("ta", "Tamil", "தமிழ்", &["tam", "tamil"]),
    ("ps", "Pashto", "پښتو", &["pus", "pashto"]),
    ("tg", "Tajik", "Тоҷикӣ", &["tgk", "tajik"]),
];

const UNKNOWN: &[&str] = &["und", "unk", "unknown", "mis", "mul", "zxx", ""];

fn aliases() -> &'static HashMap<String, &'static str> {
    static A: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    A.get_or_init(|| {
        let mut m = HashMap::new();
        for (code, _, _, al) in LANGS {
            m.insert(code.to_string(), *code);
            for a in *al {
                m.insert(a.to_string(), *code);
            }
        }
        m
    })
}

/// ISO 639-1 code for a code or name, or None if unknown.
pub fn normalize(code: Option<&str>) -> Option<&'static str> {
    let c = code?.trim().to_lowercase().replace('_', "-");
    if UNKNOWN.contains(&c.as_str()) {
        return None;
    }
    let a = aliases();
    if let Some(x) = a.get(&c) {
        return Some(x);
    }
    let base = c.split('-').next().unwrap_or("");
    a.get(base).copied()
}

pub fn same(a: Option<&str>, b: Option<&str>) -> bool {
    match (normalize(a), normalize(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

pub fn english_name(code: Option<&str>) -> String {
    match normalize(code) {
        Some(n) => LANGS.iter().find(|l| l.0 == n).map(|l| l.1.to_string()).unwrap_or_default(),
        None => code.unwrap_or("").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalises() {
        assert_eq!(normalize(Some("per")), Some("fa"));
        assert_eq!(normalize(Some("Farsi")), Some("fa"));
        assert_eq!(normalize(Some("en-US")), Some("en"));
        assert_eq!(normalize(Some("pt_BR")), Some("pt"));
        assert_eq!(normalize(Some("und")), None);
        assert!(same(Some("eng"), Some("English")));
        assert!(!same(None, None));
    }
}

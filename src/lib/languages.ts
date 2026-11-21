import { i18n } from "./i18n.svelte";

// code -> [English, Persian]
const L: Record<string, [string, string]> = {
  en: ["English", "انگلیسی"],
  fa: ["Persian", "فارسی"],
  ar: ["Arabic", "عربی"],
  fr: ["French", "فرانسوی"],
  de: ["German", "آلمانی"],
  es: ["Spanish", "اسپانیایی"],
  it: ["Italian", "ایتالیایی"],
  pt: ["Portuguese", "پرتغالی"],
  ru: ["Russian", "روسی"],
  tr: ["Turkish", "ترکی"],
  zh: ["Chinese", "چینی"],
  ja: ["Japanese", "ژاپنی"],
  ko: ["Korean", "کره‌ای"],
  hi: ["Hindi", "هندی"],
  ur: ["Urdu", "اردو"],
  he: ["Hebrew", "عبری"],
  nl: ["Dutch", "هلندی"],
  sv: ["Swedish", "سوئدی"],
  no: ["Norwegian", "نروژی"],
  da: ["Danish", "دانمارکی"],
  fi: ["Finnish", "فنلاندی"],
  pl: ["Polish", "لهستانی"],
  cs: ["Czech", "چکی"],
  el: ["Greek", "یونانی"],
  hu: ["Hungarian", "مجاری"],
  ro: ["Romanian", "رومانیایی"],
  id: ["Indonesian", "اندونزیایی"],
  ms: ["Malay", "مالایی"],
  th: ["Thai", "تایلندی"],
  vi: ["Vietnamese", "ویتنامی"],
  uk: ["Ukrainian", "اوکراینی"],
  ku: ["Kurdish", "کردی"],
  az: ["Azerbaijani", "آذربایجانی"],
  hy: ["Armenian", "ارمنی"],
  ka: ["Georgian", "گرجی"],
  bg: ["Bulgarian", "بلغاری"],
  hr: ["Croatian", "کرواتی"],
  sr: ["Serbian", "صربی"],
  ps: ["Pashto", "پشتو"],
  tg: ["Tajik", "تاجیکی"],
};

const ALIAS: Record<string, string> = {
  eng: "en", per: "fa", fas: "fa", farsi: "fa", persian: "fa", ara: "ar", fre: "fr", fra: "fr", ger: "de", deu: "de", spa: "es", ita: "it",
  por: "pt", pob: "pt", rus: "ru", tur: "tr", chi: "zh", zho: "zh", jpn: "ja", kor: "ko", hin: "hi", urd: "ur", heb: "he", dut: "nl",
  nld: "nl", swe: "sv", nor: "no", nob: "no", dan: "da", fin: "fi", pol: "pl", cze: "cs", ces: "cs", gre: "el", ell: "el", hun: "hu",
  rum: "ro", ron: "ro", ind: "id", may: "ms", msa: "ms", tha: "th", vie: "vi", ukr: "uk", kur: "ku", aze: "az", arm: "hy", hye: "hy",
  geo: "ka", kat: "ka", bul: "bg", hrv: "hr", srp: "sr", pus: "ps", tgk: "tg", english: "en", arabic: "ar",
};

export function normLang(code?: string | null): string | null {
  if (!code) return null;
  const c = code.toLowerCase().replace("_", "-");
  if (L[c]) return c;
  if (ALIAS[c]) return ALIAS[c];
  const base = c.split("-")[0];
  return L[base] ? base : ALIAS[base] ?? null;
}

export function languageName(code?: string | null): string {
  const n = normLang(code);
  if (!n) return code ?? "";
  return L[n][i18n.lang === "fa" ? 1 : 0];
}

export const LANGUAGE_CODES = Object.keys(L);

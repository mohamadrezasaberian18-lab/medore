// Translations: English text is the key; Persian comes from locales/fa.ts.
import fa from "../locales/fa";

export const i18n = $state({ lang: "en" as "en" | "fa" });

export function setLanguage(pref: string) {
  const sys = (navigator.language || "en").toLowerCase();
  const lang = pref === "auto" ? (sys.startsWith("fa") ? "fa" : "en") : pref === "fa" ? "fa" : "en";
  i18n.lang = lang;
  document.documentElement.lang = lang;
  document.documentElement.dir = lang === "fa" ? "rtl" : "ltr";
}

/** Translate; `{name}` placeholders are filled from `vars`. */
export function t(key: string, vars?: Record<string, string | number>): string {
  let s = i18n.lang === "fa" ? (fa as Record<string, string>)[key] ?? key : key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) s = s.split(`{${k}}`).join(i18n.lang === "fa" && typeof v === "number" ? toFaDigits(String(v)) : String(v));
  }
  return s;
}

/** Plural-aware count: t1("{n} episode", "{n} episodes", n) */
export function tn(one: string, many: string, n: number): string {
  return t(n === 1 ? one : many, { n });
}

export function toFaDigits(s: string): string {
  return s.replace(/[0-9]/g, (d) => "۰۱۲۳۴۵۶۷۸۹"[+d]);
}

/** Digits in the interface language. */
export function num(s: string | number): string {
  return i18n.lang === "fa" ? toFaDigits(String(s)) : String(s);
}

export function isRtl(): boolean {
  return i18n.lang === "fa";
}

/** Wrap text in a Unicode isolate so mixed Persian/English never reorders. */
export function iso(s: string): string {
  return "⁨" + s + "⁩";
}

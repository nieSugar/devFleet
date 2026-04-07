import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { tauriAPI } from "../lib/tauri";

const STORAGE_KEY = "devfleet-lang";
const DEFAULT_LANGUAGE = "zh-CN";

type LocaleMessages = Record<string, unknown>;
type LocaleModule = { default: LocaleMessages };

const localeModules = import.meta.glob("./locales/*.json", {
  eager: true,
}) as Record<string, LocaleModule>;
const resources = Object.fromEntries(
  Object.entries(localeModules).map(([filePath, mod]) => {
    const locale = filePath.replace("./locales/", "").replace(".json", "");
    return [locale, { translation: mod.default }];
  })
);

const SUPPORTED_LANGUAGES = Object.keys(resources).sort((left, right) => {
  if (left === DEFAULT_LANGUAGE) return -1;
  if (right === DEFAULT_LANGUAGE) return 1;
  return left.localeCompare(right);
});

function resolveLanguage(candidate: string | null | undefined): string | null {
  if (!candidate) return null;
  if (SUPPORTED_LANGUAGES.includes(candidate)) return candidate;

  const normalized = candidate.toLowerCase();
  const matched = SUPPORTED_LANGUAGES.find(
    (locale) =>
      locale.toLowerCase().startsWith(`${normalized}-`) ||
      normalized.startsWith(`${locale.toLowerCase()}-`)
  );

  return matched || null;
}

function getInitialLanguage(): string {
  const stored = localStorage.getItem(STORAGE_KEY);
  const storedLanguage = resolveLanguage(stored);
  if (storedLanguage) return storedLanguage;

  const nav = navigator.language;
  const browserLanguage = resolveLanguage(nav) || resolveLanguage(nav.split("-")[0]);
  return browserLanguage || DEFAULT_LANGUAGE;
}

async function syncNativeLanguage(language: string) {
  try {
    await tauriAPI.syncAppLanguage(language);
  } catch {
    // 浏览器预览或非 Tauri 运行时会走到这里，忽略即可。
  }
}

i18n.use(initReactI18next).init({
  resources,
  lng: getInitialLanguage(),
  fallbackLng: DEFAULT_LANGUAGE,
  interpolation: { escapeValue: false },
});

i18n.on("languageChanged", (lng) => {
  localStorage.setItem(STORAGE_KEY, lng);
  document.documentElement.lang = lng;
  void syncNativeLanguage(lng);
});

document.documentElement.lang = i18n.language;
void syncNativeLanguage(i18n.language);

export function getSupportedLanguages() {
  return SUPPORTED_LANGUAGES;
}

export default i18n;

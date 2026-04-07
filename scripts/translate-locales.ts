import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

type TranslationValue =
  | string
  | TranslationValue[]
  | {
      [key: string]: TranslationValue;
    };

type TranslationTree = {
  [key: string]: TranslationValue;
};

type TranslationTarget = {
  locale: string;
  targetLanguage: string;
  formality?: "default" | "more" | "less" | "prefer_more" | "prefer_less";
};

type TranslationConfig = {
  sourceLocale?: string;
  sourceLanguage?: string;
  apiBaseUrl?: string;
  targets?: TranslationTarget[];
};

type TranslateChunkParams = {
  apiKey: string;
  apiBaseUrl: string;
  sourceLanguage?: string;
  targetLanguage: string;
  texts: string[];
  formality?: TranslationTarget["formality"];
};

type Cursor = { index: number };
type DeepLResponse = {
  translations?: Array<{ text: string }>;
};

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, "..");
const LOCALES_DIR = path.join(ROOT_DIR, "src", "i18n", "locales");
const CONFIG_PATH = path.join(ROOT_DIR, "src", "i18n", "translation.config.json");
const ENV_FILES = [".env", ".env.local"];
const DEFAULT_API_BASE_URL = "https://api-free.deepl.com";
const MAX_REQUEST_BYTES = 100_000;
const PLACEHOLDER_PATTERN = /{{\s*[^{}]+\s*}}/g;

function printUsage() {
  console.log(
    `
Usage:
  pnpm i18n:translate
  pnpm i18n:translate -- en-US
  pnpm i18n:translate -- --dry-run

Environment:
  DEEPL_API_KEY       DeepL API key (required for real translation)
  DEEPL_API_BASE_URL  Optional API base URL, e.g. https://api.deepl.com
`.trim()
  );
}

function parseArgs(argv: string[]) {
  const locales: string[] = [];
  let dryRun = false;
  let help = false;

  for (const arg of argv) {
    if (arg === "--dry-run") {
      dryRun = true;
      continue;
    }
    if (arg === "--help" || arg === "-h") {
      help = true;
      continue;
    }
    locales.push(arg);
  }

  return { locales, dryRun, help };
}

async function readJson<T>(filePath: string): Promise<T> {
  const raw = await readFile(filePath, "utf8");
  return JSON.parse(raw) as T;
}

async function loadEnvFile(filePath: string) {
  let raw = "";

  try {
    raw = await readFile(filePath, "utf8");
  } catch (error: unknown) {
    const code =
      typeof error === "object" && error && "code" in error ? error.code : null;
    if (code === "ENOENT") return;
    throw error;
  }

  for (const rawLine of raw.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;

    const equalIndex = line.indexOf("=");
    if (equalIndex <= 0) continue;

    const key = line.slice(0, equalIndex).trim();
    if (!key || process.env[key]) continue;

    let value = line.slice(equalIndex + 1).trim();

    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    process.env[key] = value;
  }
}

async function loadTranslationEnv() {
  for (const envFile of ENV_FILES) {
    await loadEnvFile(path.join(ROOT_DIR, envFile));
  }
}

function isPlainObject(value: unknown): value is Record<string, TranslationValue> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function collectLeafStrings(value: TranslationValue, result: string[] = []): string[] {
  if (typeof value === "string") {
    result.push(value);
    return result;
  }

  if (Array.isArray(value)) {
    for (const item of value) collectLeafStrings(item, result);
    return result;
  }

  if (isPlainObject(value)) {
    for (const nested of Object.values(value)) collectLeafStrings(nested, result);
  }

  return result;
}

function rebuildTranslatedTree(
  source: TranslationValue,
  translations: string[],
  cursor: Cursor = { index: 0 }
): TranslationValue {
  if (typeof source === "string") {
    const translated = translations[cursor.index];
    cursor.index += 1;
    return translated ?? source;
  }

  if (Array.isArray(source)) {
    return source.map((item) => rebuildTranslatedTree(item, translations, cursor));
  }

  if (isPlainObject(source)) {
    const result: TranslationTree = {};
    for (const [key, value] of Object.entries(source)) {
      result[key] = rebuildTranslatedTree(value, translations, cursor);
    }
    return result;
  }

  return source;
}

function escapeXml(text: string) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function decodeXml(text: string) {
  return text
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&apos;", "'")
    .replaceAll("&amp;", "&");
}

function wrapPlaceholdersForDeepL(text: string) {
  let xml = "";
  let lastIndex = 0;

  // 把 i18n 插值占位符包进 XML 标签，避免 DeepL 改写 {{name}} 这类模板变量。
  for (const match of text.matchAll(PLACEHOLDER_PATTERN)) {
    const placeholder = match[0];
    const start = match.index ?? 0;
    xml += escapeXml(text.slice(lastIndex, start));
    xml += `<ph>${escapeXml(placeholder)}</ph>`;
    lastIndex = start + placeholder.length;
  }

  xml += escapeXml(text.slice(lastIndex));
  return `<root>${xml}</root>`;
}

function unwrapPlaceholdersFromDeepL(text: string) {
  const placeholders: string[] = [];

  // DeepL 返回的是 XML 文本，这里先取回占位符，再统一做 XML 反转义。
  let output = text.replace(/<ph>([\s\S]*?)<\/ph>/g, (_, content: string) => {
    const token = `__DEEPL_PLACEHOLDER_${placeholders.length}__`;
    placeholders.push(decodeXml(content));
    return token;
  });

  output = output.replace(/^<root>/, "").replace(/<\/root>$/, "");
  output = decodeXml(output);

  for (const [index, placeholder] of placeholders.entries()) {
    output = output.replace(`__DEEPL_PLACEHOLDER_${index}__`, placeholder);
  }

  return output;
}

function chunkTexts(texts: string[]) {
  const chunks: string[][] = [];
  let current: string[] = [];
  let currentSize = 0;

  for (const text of texts) {
    const size = Buffer.byteLength(text, "utf8");
    if (current.length > 0 && currentSize + size > MAX_REQUEST_BYTES) {
      chunks.push(current);
      current = [];
      currentSize = 0;
    }

    current.push(text);
    currentSize += size;
  }

  if (current.length > 0) {
    chunks.push(current);
  }

  return chunks;
}

async function translateChunk({
  apiKey,
  apiBaseUrl,
  sourceLanguage,
  targetLanguage,
  texts,
  formality,
}: TranslateChunkParams): Promise<string[]> {
  const params = new URLSearchParams();
  if (sourceLanguage && sourceLanguage !== "auto") {
    params.set("source_lang", sourceLanguage);
  }
  params.set("target_lang", targetLanguage);
  params.set("tag_handling", "xml");
  params.set("ignore_tags", "ph");

  if (formality) {
    params.set("formality", formality);
  }

  for (const text of texts) {
    params.append("text", wrapPlaceholdersForDeepL(text));
  }

  const response = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/v2/translate`, {
    method: "POST",
    headers: {
      Authorization: `DeepL-Auth-Key ${apiKey}`,
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body: params.toString(),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`DeepL request failed (${response.status}): ${errorText}`);
  }

  const data = (await response.json()) as DeepLResponse;
  return (data.translations ?? []).map((item) => unwrapPlaceholdersFromDeepL(item.text));
}

async function translateTexts(params: TranslateChunkParams): Promise<string[]> {
  const chunks = chunkTexts(params.texts);
  const results: string[] = [];

  for (const chunk of chunks) {
    const translated = await translateChunk({
      ...params,
      texts: chunk,
    });
    results.push(...translated);
  }

  return results;
}

async function main() {
  await loadTranslationEnv();

  const { locales, dryRun, help } = parseArgs(process.argv.slice(2));
  if (help) {
    printUsage();
    return;
  }

  const config = await readJson<TranslationConfig>(CONFIG_PATH);
  const sourceLocale = config.sourceLocale ?? "zh-CN";
  const sourceLanguage =
    config.sourceLanguage && config.sourceLanguage !== "auto"
      ? config.sourceLanguage
      : undefined;
  const apiBaseUrl =
    process.env.DEEPL_API_BASE_URL || config.apiBaseUrl || DEFAULT_API_BASE_URL;
  const targets = Array.isArray(config.targets) ? config.targets : [];
  const localeFilter = new Set(locales);
  const selectedTargets =
    localeFilter.size > 0
      ? targets.filter((target) => localeFilter.has(target.locale))
      : targets;

  if (selectedTargets.length === 0) {
    throw new Error(
      "No translation targets matched. Update translation.config.json or pass a valid locale."
    );
  }

  const sourcePath = path.join(LOCALES_DIR, `${sourceLocale}.json`);
  const sourceTranslations = await readJson<TranslationTree>(sourcePath);
  const sourceTexts = collectLeafStrings(sourceTranslations);

  console.log(`Source locale: ${sourceLocale}`);
  console.log(`Source file: ${path.relative(ROOT_DIR, sourcePath)}`);
  console.log(`String count: ${sourceTexts.length}`);

  if (dryRun) {
    for (const target of selectedTargets) {
      const outputPath = path.join(LOCALES_DIR, `${target.locale}.json`);
      console.log(
        `[dry-run] ${target.locale} -> ${path.relative(ROOT_DIR, outputPath)} (${target.targetLanguage})`
      );
    }
    return;
  }

  const apiKey = process.env.DEEPL_API_KEY;
  if (!apiKey) {
    throw new Error(
      "DEEPL_API_KEY is required. You can copy .env.example and fill in the key first."
    );
  }

  await mkdir(LOCALES_DIR, { recursive: true });

  for (const target of selectedTargets) {
    if (target.locale === sourceLocale) {
      console.log(`Skip ${target.locale}: target locale is the source locale.`);
      continue;
    }

    if (!target.targetLanguage) {
      throw new Error(
        `Target ${target.locale} is missing targetLanguage in translation.config.json.`
      );
    }

    const outputPath = path.join(LOCALES_DIR, `${target.locale}.json`);
    console.log(
      `Translating ${sourceLocale} -> ${target.locale} (${target.targetLanguage})`
    );

    const translatedTexts = await translateTexts({
      apiKey,
      apiBaseUrl,
      sourceLanguage,
      targetLanguage: target.targetLanguage,
      texts: sourceTexts,
      formality: target.formality,
    });

    if (translatedTexts.length !== sourceTexts.length) {
      throw new Error(
        `DeepL returned ${translatedTexts.length} items, expected ${sourceTexts.length} for ${target.locale}.`
      );
    }

    const translatedTree = rebuildTranslatedTree(
      sourceTranslations,
      translatedTexts
    ) as TranslationTree;
    await writeFile(outputPath, `${JSON.stringify(translatedTree, null, 2)}\n`, "utf8");
    console.log(`Wrote ${path.relative(ROOT_DIR, outputPath)}`);
  }
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[i18n] ${message}`);
  process.exit(1);
});

import fs from "node:fs";
import path from "node:path";

type PackageJson = {
  version?: string;
};

type TauriConfig = {
  version?: string;
};

const repoRoot = path.resolve(import.meta.dirname, "..");
const packageJsonPath = path.join(repoRoot, "package.json");
const cargoTomlPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
const tauriConfigPath = path.join(repoRoot, "src-tauri", "tauri.conf.json");

function readJsonFile<T>(filePath: string): T {
  return JSON.parse(fs.readFileSync(filePath, "utf8")) as T;
}

function writeJsonFile(filePath: string, value: unknown) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function updateCargoVersion(cargoToml: string, version: string) {
  return cargoToml.replace(
    /^version = ".*"$/m,
    `version = "${version}"`,
  );
}

function main() {
  const packageJson = readJsonFile<PackageJson>(packageJsonPath);
  const nextVersion = packageJson.version?.trim();

  if (!nextVersion) {
    throw new Error("package.json 未配置 version，无法同步桌面端版本号。");
  }

  const tauriConfig = readJsonFile<TauriConfig>(tauriConfigPath);
  if (tauriConfig.version !== nextVersion) {
    tauriConfig.version = nextVersion;
    writeJsonFile(tauriConfigPath, tauriConfig);
  }

  const cargoToml = fs.readFileSync(cargoTomlPath, "utf8");
  const updatedCargoToml = updateCargoVersion(cargoToml, nextVersion);
  if (updatedCargoToml !== cargoToml) {
    fs.writeFileSync(cargoTomlPath, updatedCargoToml);
  }

  console.log(`[sync-app-version] synced version -> ${nextVersion}`);
}

main();

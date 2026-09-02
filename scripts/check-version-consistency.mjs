import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

async function readJson(path) {
  return JSON.parse(await readFile(join(projectRoot, path), "utf8"));
}

function packageVersionFromToml(contents, path) {
  const packageTable = contents.match(/^\[package\]\s*\n([\s\S]*?)(?=\n\[|$)/);
  const version = packageTable?.[1].match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!version) {
    throw new Error(`Could not read [package].version from ${path}.`);
  }
  return version;
}

const [packageJson, packageLock, tauriConfig, chromeManifest, cargoToml, cargoLock] =
  await Promise.all([
    readJson("package.json"),
    readJson("package-lock.json"),
    readJson("src-tauri/tauri.conf.json"),
    readJson("chrome-extension/manifest.json"),
    readFile(join(projectRoot, "src-tauri", "Cargo.toml"), "utf8"),
    readFile(join(projectRoot, "src-tauri", "Cargo.lock"), "utf8"),
  ]);

const cargoLockVersion = cargoLock.match(
  /\[\[package\]\]\s*\nname = "legra"\s*\nversion = "([^"]+)"/,
)?.[1];
if (!cargoLockVersion) {
  throw new Error("Could not read the Legra package version from src-tauri/Cargo.lock.");
}

const expected = packageJson.version;
const versions = new Map([
  ["package.json", packageJson.version],
  ["package-lock.json", packageLock.version],
  ["package-lock.json packages['']", packageLock.packages?.[""]?.version],
  ["src-tauri/Cargo.toml", packageVersionFromToml(cargoToml, "src-tauri/Cargo.toml")],
  ["src-tauri/Cargo.lock", cargoLockVersion],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
  ["chrome-extension/manifest.json", chromeManifest.version],
]);

const mismatches = [...versions].filter(([, version]) => version !== expected);
if (mismatches.length > 0) {
  throw new Error(
    `Version mismatch; expected ${expected}:\n${mismatches
      .map(([path, version]) => `- ${path}: ${version ?? "missing"}`)
      .join("\n")}`,
  );
}

if (process.env.GITHUB_REF_TYPE === "tag") {
  const expectedTag = `v${expected}`;
  if (process.env.GITHUB_REF_NAME !== expectedTag) {
    throw new Error(
      `Release tag ${process.env.GITHUB_REF_NAME ?? "missing"} does not match ${expectedTag}.`,
    );
  }
}

process.stdout.write(`Legra version consistency check passed: ${expected}\n`);

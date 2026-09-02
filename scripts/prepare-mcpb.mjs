import { chmod, copyFile, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDirectory, "..");

function usage() {
  return "Usage: node scripts/prepare-mcpb.mjs <binary> <darwin|win32> <output-directory>";
}

const [binaryArgument, platform, outputArgument] = process.argv.slice(2);
if (!binaryArgument || !outputArgument || !["darwin", "win32"].includes(platform)) {
  throw new Error(usage());
}

const binaryPath = resolve(binaryArgument);
const outputDirectory = resolve(outputArgument);
const packageJson = JSON.parse(
  await readFile(join(projectRoot, "package.json"), "utf8"),
);
const executableName = platform === "win32" ? "legra_mcp.exe" : "legra_mcp";
const bundledExecutable = join(outputDirectory, "server", executableName);

await rm(outputDirectory, { recursive: true, force: true });
await mkdir(dirname(bundledExecutable), { recursive: true });
await copyFile(binaryPath, bundledExecutable);
if (platform !== "win32") {
  await chmod(bundledExecutable, 0o755);
}
await copyFile(join(projectRoot, "src-tauri", "icons", "icon.png"), join(outputDirectory, "icon.png"));

const tools = [
  ["resolve_paper", "Normalize a DOI or arXiv identifier and check whether it is already in Legra."],
  ["search_library", "Search papers already registered in the local Legra library."],
  ["list_categories", "List the folder categories currently available in Legra."],
  ["import_paper", "Queue a DOI or arXiv paper for validated import by the Legra desktop app."],
  ["get_import_status", "Check the status and result of a queued Legra import job."],
].map(([name, description]) => ({ name, description }));

const manifest = {
  manifest_version: "0.3",
  name: "legra-paper-importer",
  display_name: "Legra Paper Importer",
  version: packageJson.version,
  description: "Import papers found by Claude into the local-first Legra paper library.",
  long_description:
    "Queues DOI and arXiv identifiers for Legra to validate, download when permitted, organize, and register. Legra remains the only process that writes the paper catalog.",
  author: {
    name: "Koki Mizuno",
  },
  repository: {
    type: "git",
    url: "https://github.com/KokiMizuno531/legra",
  },
  icon: "icon.png",
  server: {
    type: "binary",
    entry_point: `server/${executableName}`,
    mcp_config: {
      command: `\${__dirname}/server/${executableName}`,
      args: [],
      env: {},
    },
  },
  tools,
  tools_generated: false,
  keywords: ["research", "papers", "DOI", "arXiv", "local-first"],
  license: "MIT",
  compatibility: {
    claude_desktop: ">=1.0.0",
    platforms: [platform],
  },
};

await writeFile(
  join(outputDirectory, "manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
  "utf8",
);

process.stdout.write(
  `Prepared Claude Desktop MCPB directory for ${platform}: ${outputDirectory} (${basename(binaryPath)})\n`,
);

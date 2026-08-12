import { basename } from "node:path";
import { readFile, writeFile } from "node:fs/promises";

const [version, archivePath, signaturePath, outputPath] = process.argv.slice(2);

if (!version || !archivePath || !signaturePath || !outputPath) {
  throw new Error(
    "Usage: generate-update-manifest.mjs <version> <archive> <signature> <output>",
  );
}

const signature = (await readFile(signaturePath, "utf8")).trim();
const baseUrl =
  process.env.UPDATE_DOWNLOAD_BASE_URL ??
  `https://github.com/idaibin/ai-quota-trends/releases/download/v${version}`;
const archiveUrl = `${baseUrl.replace(/\/$/, "")}/${encodeURIComponent(basename(archivePath))}`;

const manifest = {
  version,
  notes: process.env.UPDATE_NOTES ?? `AI Quota Trends ${version}`,
  pub_date: new Date().toISOString(),
  platforms: {
    "darwin-universal": {
      signature,
      url: archiveUrl,
    },
  },
};

await writeFile(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

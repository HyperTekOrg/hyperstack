#!/usr/bin/env node

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const pkg = require("../package.json");
const version = pkg.version;

const PLATFORMS = {
  "darwin-arm64": "hs-darwin-arm64",
  "darwin-x64": "hs-darwin-x64",
  "linux-x64": "hs-linux-x64",
  "linux-arm64": "hs-linux-arm64",
  "win32-x64": "hs-win32-x64.exe",
};

const platform = process.platform;
const arch = process.arch;
const key = `${platform}-${arch}`;

const binaryName = PLATFORMS[key];
if (!binaryName) {
  console.warn(
    `Hyperstack CLI does not have a prebuilt binary for ${key}.\n` +
    "You can build from source: cargo install hyperstack-cli"
  );
  process.exit(0);
}

const binDir = path.join(__dirname, "..", "bin");
const binPath = path.join(binDir, platform === "win32" ? "hs.exe" : "hs");

if (fs.existsSync(binPath)) {
  process.exit(0);
}

const url = `https://github.com/HyperTekOrg/hyperstack/releases/download/hyperstack-cli-v${version}/${binaryName}`;

console.log(`Downloading Hyperstack CLI v${version} for ${key}...`);

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest, { mode: 0o755 });
    
    const request = (url) => {
      https.get(url, (response) => {
        if (response.statusCode === 302 || response.statusCode === 301) {
          request(response.headers.location);
          return;
        }
        
        if (response.statusCode !== 200) {
          fs.unlinkSync(dest);
          reject(new Error(`Download failed: HTTP ${response.statusCode}`));
          return;
        }
        
        const total = parseInt(response.headers["content-length"], 10);
        let downloaded = 0;
        
        response.on("data", (chunk) => {
          downloaded += chunk.length;
          if (total && process.stdout.isTTY) {
            const pct = Math.round((downloaded / total) * 100);
            process.stdout.write(`\rDownloading... ${pct}%`);
          }
        });
        
        response.pipe(file);
        
        file.on("finish", () => {
          file.close();
          if (process.stdout.isTTY) {
            process.stdout.write("\n");
          }
          resolve();
        });
      }).on("error", (err) => {
        fs.unlinkSync(dest);
        reject(err);
      });
    };
    
    request(url);
  });
}

async function main() {
  try {
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }
    
    await download(url, binPath);
    
    if (platform !== "win32") {
      fs.chmodSync(binPath, 0o755);
    }
    
    console.log("Hyperstack CLI installed successfully.");
  } catch (err) {
    console.error(`\nFailed to download Hyperstack CLI: ${err.message}`);
    console.error(
      "\nYou can install manually via Cargo:\n" +
      "  cargo install hyperstack-cli\n" +
      "\nOr download directly from:\n" +
      `  ${url}`
    );
    process.exit(0);
  }
}

main();

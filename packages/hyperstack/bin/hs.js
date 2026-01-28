#!/usr/bin/env node

const { spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const binName = process.platform === "win32" ? "hs.exe" : "hs";
const binPath = path.join(__dirname, binName);

if (!fs.existsSync(binPath)) {
  console.error(
    "Hyperstack CLI binary not found. This usually means the postinstall script failed.\n" +
    "Try reinstalling: npm install hyperstack-cli\n" +
    "\n" +
    "If the problem persists, you can install the CLI via Cargo:\n" +
    "  cargo install hyperstack-cli"
  );
  process.exit(1);
}

const result = spawnSync(binPath, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env,
});

if (result.error) {
  if (result.error.code === "EACCES") {
    console.error(
      "Permission denied. Try running:\n" +
      `  chmod +x "${binPath}"`
    );
  } else {
    console.error("Failed to run Hyperstack CLI:", result.error.message);
  }
  process.exit(1);
}

process.exit(result.status ?? 1);

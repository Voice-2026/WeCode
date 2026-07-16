#!/usr/bin/env node
/* global console, process */

import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "wecode-cask-test-"));
const caskPath = path.join(tempDir, "wecode.rb");

try {
  const result = spawnSync(
    "node",
    [
      "apps/desktop/scripts/release/render-homebrew-cask.mjs",
      "v1.8.0",
      "arm-sha",
      caskPath,
    ],
    { cwd: root, stdio: "pipe", encoding: "utf8" },
  );

  if (result.status !== 0) {
    process.stdout.write(result.stdout || "");
    process.stderr.write(result.stderr || "");
    process.exit(result.status ?? 1);
  }

  const cask = fs.readFileSync(caskPath, "utf8");
  assert.match(cask, /sha256 "arm-sha"/);
  assert.match(cask, /wecode-#\{version\}-macos-aarch64\.dmg/);
  assert.match(cask, /depends_on arch: :arm64/);
  assert.match(cask, /Contents\/Resources\/bin\/wecode/);
  assert.match(cask, /target: "wecode"/);
  assert.match(cask, /Voice-2026\/WeCode/);
  assert.doesNotMatch(cask, /github\.com\/duxweb/);
  assert.doesNotMatch(cask, /on_intel do/);
  assert.doesNotMatch(cask, /macos-universal-formal/);
} finally {
  fs.rmSync(tempDir, { recursive: true, force: true });
}

console.log("homebrew cask render test passed");

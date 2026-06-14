#!/usr/bin/env node
/* global console, process */

import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "codux-prepare-release-test-"));
const notesPath = path.join(tempDir, "release-notes.md");

const result = spawnSync("node", ["apps/desktop/scripts/release/prepare-release.mjs", "v1.8.0", "--dry-run"], {
  cwd: root,
  env: {
    ...process.env,
    RELEASE_NOTES_PATH: notesPath,
  },
  encoding: "utf8",
});

assert.equal(result.status, 0, result.stderr || result.stdout);
const notes = fs.readFileSync(notesPath, "utf8");
assert.match(notes, /## 下载说明/);
assert.match(notes, /codux-\*-macos-aarch64\.dmg/);
assert.match(notes, /codux-\*-macos-x86_64\.dmg/);
assert.match(notes, /codux-\*-macos-aarch64-debug\.dmg/);
assert.match(notes, /codux-\*-macos-x86_64-debug\.dmg/);
assert.match(notes, /codux-\*-windows-x86_64-setup\.exe/);
assert.match(notes, /\| `codux-\*-macos-aarch64\.dmg` \| Apple Silicon Mac 正式版本 \|/);
assert.match(notes, /\| `codux-\*-macos-x86_64\.dmg` \| Intel Mac 正式版本 \|/);
assert.match(notes, /\| `codux-\*-macos-aarch64-debug\.dmg` \| Apple Silicon Mac 测试版本 \|/);
assert.match(notes, /\| `codux-\*-macos-x86_64-debug\.dmg` \| Intel Mac 测试版本 \|/);
assert.doesNotMatch(notes, /latest\.json/);
assert.doesNotMatch(notes, /updater\.app\.tar\.gz/);

fs.rmSync(tempDir, { recursive: true, force: true });
console.log("prepare-release notes test passed");

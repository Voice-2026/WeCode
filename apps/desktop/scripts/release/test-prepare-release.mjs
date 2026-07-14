#!/usr/bin/env node
/* global console, process */

import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "wecode-prepare-release-test-"));
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
assert.match(notes, /## Downloads \/ 下载说明/);
assert.match(notes, /\| File \/ 文件 \| Usage \| 用途 \|/);
assert.match(
  notes,
  /\| \[`wecode-1\.8\.0-macos-aarch64\.dmg`\]\(https:\/\/github\.com\/duxweb\/wecode\/releases\/download\/v1\.8\.0\/wecode-1\.8\.0-macos-aarch64\.dmg\) \| Apple Silicon Mac stable release \| Apple Silicon Mac 正式版本 \|/,
);
assert.doesNotMatch(notes, /macos-x86_64/);
assert.doesNotMatch(notes, /windows/i);
assert.doesNotMatch(notes, /debug/i);
assert.doesNotMatch(notes, /wecode-agent/i);
assert.doesNotMatch(notes, /wecode-\*/);
assert.doesNotMatch(notes, /latest\.json/);
assert.doesNotMatch(notes, /updater\.app\.tar\.gz/);

const rcChannelResult = spawnSync(
  "node",
  ["apps/desktop/scripts/release/prepare-release.mjs", "v2.0.0-rc.1", "--dry-run"],
  {
    cwd: root,
    env: {
      ...process.env,
      RELEASE_NOTES_PATH: path.join(tempDir, "rc-release-notes.md"),
    },
    encoding: "utf8",
  },
);

assert.equal(rcChannelResult.status, 0, rcChannelResult.stderr || rcChannelResult.stdout);
assert.match(rcChannelResult.stdout, /^channel=stable$/m);

const missingNotesResult = spawnSync(
  "node",
  ["apps/desktop/scripts/release/prepare-release.mjs", "v9.9.9-beta.99", "--dry-run"],
  {
    cwd: root,
    env: {
      ...process.env,
      RELEASE_NOTES_PATH: path.join(tempDir, "missing-release-notes.md"),
    },
    encoding: "utf8",
  },
);

assert.notEqual(missingNotesResult.status, 0);
assert.match(
  missingNotesResult.stderr,
  /Missing release notes for 9\.9\.9-beta\.99/,
);
assert.ok(!fs.existsSync(path.join(tempDir, "missing-release-notes.md")));

fs.rmSync(tempDir, { recursive: true, force: true });
console.log("prepare-release notes test passed");

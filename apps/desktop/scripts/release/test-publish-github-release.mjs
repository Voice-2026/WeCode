#!/usr/bin/env node
/* global console, process */

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = process.cwd();
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "wecode-release-test-"));
const artifactsDir = path.join(tempDir, "artifacts");
const notesPath = path.join(tempDir, "notes.md");

fs.mkdirSync(path.join(artifactsDir, "macos"), { recursive: true });
fs.mkdirSync(path.join(artifactsDir, "windows"), { recursive: true });
fs.writeFileSync(notesPath, "WeCode release notes", "utf8");
writeAsset("macos/wecode-1.5.0-macos-aarch64-updater.app.tar.gz", "mac-arm");
writeAsset("macos/wecode-1.5.0-macos-aarch64-updater.app.tar.gz.sig", "mac-arm-signature");
writeAsset("macos/wecode-1.5.0-macos-aarch64-updater.app.tar.gz.sha256", "mac-arm-sha");
writeAsset("macos/wecode-1.5.0-macos-aarch64.dmg", "dmg-arm");
writeAsset("macos/wecode-1.5.0-macos-aarch64.dmg.sha256", "dmg-arm-sha");
writeAsset("macos/wecode-1.5.0-macos-x86_64-updater.app.tar.gz", "mac-intel");
writeAsset("macos/wecode-1.5.0-macos-x86_64-updater.app.tar.gz.sig", "mac-intel-signature");
writeAsset("macos/wecode-1.5.0-macos-x86_64-updater.app.tar.gz.sha256", "mac-intel-sha");
writeAsset("macos/wecode-1.5.0-macos-x86_64.dmg", "dmg-intel");
writeAsset("macos/wecode-1.5.0-macos-x86_64.dmg.sha256", "dmg-intel-sha");
writeAsset("windows/wecode-1.5.0-windows-x86_64-setup.exe", "win");
writeAsset("windows/wecode-1.5.0-windows-x86_64-setup.exe.sig", "win-signature");
writeAsset("windows/wecode-1.5.0-windows-x86_64-setup.exe.sha256", "win-sha");
writeAsset("windows/wecode-1.5.0-windows-x86_64.zip", "win-zip");
writeAsset("windows/WeCode/WeCode.exe", "raw-exe");

const result = spawnSync(
  "node",
  ["apps/desktop/scripts/release/publish-github-release.mjs", "--dry-run"],
  {
    cwd: root,
    stdio: "pipe",
    encoding: "utf8",
    env: {
      ...process.env,
      RELEASE_VERSION: "1.5.0",
      RELEASE_CHANNEL: "stable",
      RELEASE_TAG: "v1.5.0",
      RELEASE_ARTIFACTS_DIR: artifactsDir,
      RELEASE_NOTES_PATH: notesPath,
    },
  },
);

if (result.status !== 0) {
  process.stdout.write(result.stdout || "");
  process.stderr.write(result.stderr || "");
  process.exit(result.status ?? 1);
}
assert(
  result.stdout.includes("Prepared 5 public assets and update metadata"),
  `unexpected dry-run output: ${result.stdout}`,
);
assert(releaseScriptUsesStableBetaReleaseFlag(), "beta releases must not be marked as GitHub prereleases");

const manifest = JSON.parse(fs.readFileSync(path.join(artifactsDir, "latest.json"), "utf8"));
assertEqual(manifest.version, "1.5.0");
assertEqual(manifest.notes, "WeCode release notes");
assert(!("automaticInstallSupported" in manifest), "manifest must stay Tauri-compatible");
assert(!("downloadUrl" in manifest), "manifest must not contain GPUI-only downloadUrl");
assert(!("checksum" in manifest), "manifest must not contain GPUI-only checksum");

for (const key of ["darwin-aarch64", "darwin-aarch64-app"]) {
  assertEqual(manifest.platforms[key].signature, "mac-arm-signature");
  assert(manifest.platforms[key].url.includes("macos-aarch64"), `${key} should use aarch64 updater`);
  assert(manifest.platforms[key].url.endsWith(".app.tar.gz"), `${key} should use app.tar.gz`);
}

for (const key of ["darwin-x86_64", "darwin-x86_64-app"]) {
  assertEqual(manifest.platforms[key].signature, "mac-intel-signature");
  assert(manifest.platforms[key].url.includes("macos-x86_64"), `${key} should use x86_64 updater`);
  assert(manifest.platforms[key].url.endsWith(".app.tar.gz"), `${key} should use app.tar.gz`);
}

for (const key of ["windows-x86_64", "windows-x86_64-nsis"]) {
  assertEqual(manifest.platforms[key].signature, "win-signature");
  assert(manifest.platforms[key].url.endsWith(".exe"), `${key} should use NSIS exe`);
}

const unsignedArtifactsDir = path.join(tempDir, "unsigned-artifacts");
writeAssetTo(unsignedArtifactsDir, "macos/wecode-1.5.0-macos-aarch64-updater.app.tar.gz", "mac-arm");
writeAssetTo(unsignedArtifactsDir, "macos/wecode-1.5.0-macos-aarch64.dmg", "dmg-arm");

const unsignedResult = runDryRun(unsignedArtifactsDir, "false");
assertEqual(unsignedResult.status, 0);
assert(
  unsignedResult.stdout.includes("Prepared 2 public assets without updater metadata"),
  `unexpected unsigned dry-run output: ${unsignedResult.stdout}`,
);
assert(
  !fs.existsSync(path.join(unsignedArtifactsDir, "latest.json")),
  "unsigned releases must not create updater metadata",
);

const requiredSignatureResult = runDryRun(unsignedArtifactsDir, "true");
assert(requiredSignatureResult.status !== 0, "signed release mode must reject unsigned updater assets");
assert(
  requiredSignatureResult.stderr.includes("No signed updater assets found"),
  `unexpected required-signature error: ${requiredSignatureResult.stderr}`,
);

fs.rmSync(tempDir, { recursive: true, force: true });
console.log("release manifest test passed");

function releaseScriptUsesStableBetaReleaseFlag() {
  const script = fs.readFileSync(path.join(root, "apps/desktop/scripts/release/publish-github-release.mjs"), "utf8");
  return (
    script.includes('channel === "beta" ? "--latest=false" : "--latest"') &&
    !script.includes('channel === "beta" ? "--prerelease"')
  );
}

function writeAsset(relativePath, content) {
  writeAssetTo(artifactsDir, relativePath, content);
}

function writeAssetTo(baseDir, relativePath, content) {
  const assetPath = path.join(baseDir, relativePath);
  fs.mkdirSync(path.dirname(assetPath), { recursive: true });
  fs.writeFileSync(assetPath, content, "utf8");
}

function runDryRun(releaseArtifactsDir, requireSignature) {
  return spawnSync("node", ["apps/desktop/scripts/release/publish-github-release.mjs", "--dry-run"], {
    cwd: root,
    stdio: "pipe",
    encoding: "utf8",
    env: {
      ...process.env,
      RELEASE_VERSION: "1.5.0",
      RELEASE_CHANNEL: "stable",
      RELEASE_TAG: "v1.5.0",
      RELEASE_ARTIFACTS_DIR: releaseArtifactsDir,
      RELEASE_NOTES_PATH: notesPath,
      RELEASE_REQUIRE_TAURI_SIGNATURE: requireSignature,
    },
  });
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertEqual(actual, expected) {
  if (actual !== expected) {
    throw new Error(`expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

#!/usr/bin/env node
/* global console, process */

import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

process.env.WECODE_PACKAGE_GPUI_TEST_MODE = "true";
process.env.RELEASE_STAGE_DIR = "target/release-package-test";

const {
  __testIsTauriUpdaterSignatureRequired,
  __testPackageWindows,
  __testStageRuntimeAssets,
  __testWindowsNsisScript,
} = await import("./package-gpui.mjs");

const oldGithubActions = process.env.GITHUB_ACTIONS;
const oldSignatureRequirement = process.env.RELEASE_REQUIRE_TAURI_SIGNATURE;
try {
  delete process.env.GITHUB_ACTIONS;
  delete process.env.RELEASE_REQUIRE_TAURI_SIGNATURE;
  assert.equal(__testIsTauriUpdaterSignatureRequired(), false);

  process.env.GITHUB_ACTIONS = "true";
  assert.equal(__testIsTauriUpdaterSignatureRequired(), true);

  process.env.RELEASE_REQUIRE_TAURI_SIGNATURE = "false";
  assert.equal(__testIsTauriUpdaterSignatureRequired(), false);

  delete process.env.GITHUB_ACTIONS;
  process.env.RELEASE_REQUIRE_TAURI_SIGNATURE = "true";
  assert.equal(__testIsTauriUpdaterSignatureRequired(), true);
} finally {
  restoreEnvironmentVariable("GITHUB_ACTIONS", oldGithubActions);
  restoreEnvironmentVariable("RELEASE_REQUIRE_TAURI_SIGNATURE", oldSignatureRequirement);
}

const script = __testWindowsNsisScript(
  path.join("C:", "tmp", "WeCode"),
  path.join("C:", "tmp", "wecode-setup.exe"),
);

assert.match(script, /!include MUI2\.nsh/);
assert.match(script, /ManifestDPIAware true/);
assert.match(script, /VIProductVersion "\d+\.\d+\.\d+\.\d+"/);
assert.match(script, /!insertmacro MUI_PAGE_DIRECTORY/);
assert.match(script, /!insertmacro MUI_LANGUAGE "English"/);
assert.match(script, /!insertmacro MUI_LANGUAGE "SimpChinese"/);
assert.match(script, /MUI_HEADERIMAGE_BITMAP/);
assert.doesNotMatch(script, /MUI_PAGE_WELCOME/);
assert.doesNotMatch(script, /Page custom/);
assert.match(script, /Function EnsureWeCodeCanBeUpdated/);
assert.match(script, /WeCode is still running or the executable is locked/);
assert.match(script, /CreateShortcut "\$DESKTOP\\WeCode\.lnk"/);
assert.match(script, /CreateShortcut "\$SMPROGRAMS\\WeCode\\WeCode\.lnk"/);
assert.match(script, /Uninstall\\WeCode"$/m);
assert.match(script, /"UninstallString"/);
assert.match(script, /Exec '"\$INSTDIR\\WeCode\.exe"'/);
assert.match(script, /RMDir \/r "\$INSTDIR\\Data"/);
assert.match(script, /\/SD IDNO/);

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "wecode-package-runtime-"));
try {
  const runtimeRoot = path.join(tempDir, "runtime-root");
  __testStageRuntimeAssets(runtimeRoot);
  for (const relativePath of [
    "scripts/shell-hooks/dmux-ai-hook.zsh",
    "scripts/shell-hooks/zsh/.zlogin",
    "scripts/shell-hooks/zsh/.zprofile",
    "scripts/shell-hooks/zsh/.zshenv",
    "scripts/shell-hooks/zsh/.zshrc",
    "scripts/wrappers/tool-wrapper.sh",
    "scripts/wrappers/dmux-ai-state.sh",
    "scripts/wrappers/wecode-ssh.ps1",
    "scripts/wrappers/wecode-db.ps1",
    "scripts/wrappers/bin/codex",
    "scripts/wrappers/bin/wecode-ssh",
    "scripts/wrappers/bin/wecode-ssh.ps1",
    "scripts/wrappers/bin/wecode-db",
    "scripts/wrappers/bin/wecode-db.ps1",
  ]) {
    assert.equal(fs.existsSync(path.join(runtimeRoot, relativePath)), true, `${relativePath} should be packaged`);
  }
  assertNoSymlinks(runtimeRoot);
} finally {
  fs.rmSync(tempDir, { recursive: true, force: true });
}

console.log("package-gpui installer test passed");

if (process.platform === "win32") {
  const binaryDir = fs.mkdtempSync(path.join(os.tmpdir(), "wecode-package-binaries-"));
  const packageDir = fs.mkdtempSync(path.join(os.tmpdir(), "wecode-package-output-"));
  const oldBinaryDir = process.env.WECODE_RELEASE_BINARY_DIR;
  const oldTestPackageDir = process.env.WECODE_TEST_PACKAGE_DIR;
  const oldSkipMakensis = process.env.WECODE_TEST_SKIP_MAKENSIS;
  const oldSignatureRequirement = process.env.RELEASE_REQUIRE_TAURI_SIGNATURE;
  try {
    fs.writeFileSync(path.join(binaryDir, "wecode.exe"), "gui");
    fs.writeFileSync(path.join(binaryDir, "wecode-wrapper-helper.exe"), "console-helper");
    process.env.WECODE_RELEASE_BINARY_DIR = binaryDir;
    process.env.WECODE_TEST_PACKAGE_DIR = packageDir;
    process.env.WECODE_TEST_SKIP_MAKENSIS = "true";
    process.env.RELEASE_REQUIRE_TAURI_SIGNATURE = "false";
    __testPackageWindows();

    assert.equal(
      fs.readFileSync(path.join(packageDir, "runtime-root", "scripts", "wrappers", "wecode-wrapper-helper.exe"), "utf8"),
      "console-helper",
      "Windows package should include the console wrapper helper",
    );

    const outputDir = path.join(
      process.cwd(),
      process.env.RELEASE_STAGE_DIR,
      process.env.RELEASE_BUILD_ID || `${process.platform}-${process.arch}`,
    );
    assert.equal(
      fs.readdirSync(outputDir).some((name) => name.endsWith("-setup.exe")),
      true,
      "mock installer should be produced",
    );
  } finally {
    restoreEnvironmentVariable("WECODE_RELEASE_BINARY_DIR", oldBinaryDir);
    restoreEnvironmentVariable("WECODE_TEST_PACKAGE_DIR", oldTestPackageDir);
    restoreEnvironmentVariable("WECODE_TEST_SKIP_MAKENSIS", oldSkipMakensis);
    restoreEnvironmentVariable("RELEASE_REQUIRE_TAURI_SIGNATURE", oldSignatureRequirement);
    fs.rmSync(binaryDir, { recursive: true, force: true });
    fs.rmSync(packageDir, { recursive: true, force: true });
  }
}

function assertNoSymlinks(root) {
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const entryPath = path.join(root, entry.name);
    assert.equal(entry.isSymbolicLink(), false, `${entryPath} should not be a symlink`);
    if (entry.isDirectory()) {
      assertNoSymlinks(entryPath);
    }
  }
}

function restoreEnvironmentVariable(name, value) {
  if (value === undefined) {
    delete process.env[name];
  } else {
    process.env[name] = value;
  }
}

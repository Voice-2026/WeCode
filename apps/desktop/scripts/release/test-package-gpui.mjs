#!/usr/bin/env node
/* global console, process */

import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

process.env.CODUX_PACKAGE_GPUI_TEST_MODE = "true";
process.env.RELEASE_STAGE_DIR = "target/release-package-test";

const { __testStageRuntimeAssets, __testWindowsNsisScript } = await import("./package-gpui.mjs");

const script = __testWindowsNsisScript(
  path.join("C:", "tmp", "Codux"),
  path.join("C:", "tmp", "codux-setup.exe"),
);

assert.match(script, /!include MUI2\.nsh/);
assert.match(script, /!insertmacro MUI_PAGE_WELCOME/);
assert.match(script, /!insertmacro MUI_PAGE_DIRECTORY/);
assert.match(script, /Page custom OptionsPageCreate OptionsPageLeave/);
assert.match(script, /Create Start Menu shortcut/);
assert.match(script, /Create Desktop shortcut/);
assert.match(script, /Function EnsureCoduxCanBeUpdated/);
assert.match(script, /Codux is still running or the executable is locked/);
assert.match(script, /CreateShortcut "\$DESKTOP\\\\Codux\.lnk"/);
assert.match(script, /CreateShortcut "\$SMPROGRAMS\\\\Codux\\\\Codux\.lnk"/);
assert.doesNotMatch(script, /ShowInstDetails nevershow/);

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "codux-package-runtime-"));
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
    "scripts/wrappers/bin/codex",
  ]) {
    assert.equal(fs.existsSync(path.join(runtimeRoot, relativePath)), true, `${relativePath} should be packaged`);
  }
} finally {
  fs.rmSync(tempDir, { recursive: true, force: true });
}

console.log("package-gpui installer test passed");

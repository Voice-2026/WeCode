#!/usr/bin/env node
/* global process */

import fs from "node:fs";
import path from "node:path";

const [, , versionArg, armSha256, outputPath] = process.argv;
const version = (versionArg || "").replace(/^v/, "");

if (!version || !armSha256 || !outputPath) {
  throw new Error("usage: render-homebrew-cask.mjs <version> <arm-sha256> <output-path>");
}

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(
  outputPath,
  `cask "wecode" do
  version "${version}"

  sha256 "${armSha256}"

  url "https://github.com/Voice-2026/WeCode/releases/download/v#{version}/wecode-#{version}-macos-aarch64.dmg"

  name "WeCode"
  desc "Native terminal workspace for AI coding tools"
  homepage "https://github.com/Voice-2026/WeCode"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "WeCode.app"
  binary "#{appdir}/WeCode.app/Contents/Resources/bin/wecode", target: "wecode"

  zap trash: [
    "~/Library/Application Support/WeCode",
    "~/Library/Caches/com.duxweb.wecode",
    "~/Library/HTTPStorages/com.duxweb.wecode",
    "~/Library/Preferences/com.duxweb.wecode.plist",
    "~/Library/Saved Application State/com.duxweb.wecode.savedState",
  ]
end
`,
  "utf8",
);

# WeCode Agent Rules

## Current platform scope

- WeCode currently targets macOS only.
- Do not add, run, wait for, or require Windows CI, builds, packaging, testing, or releases unless the user explicitly expands the scope in a later request.
- Agent CLI releases may continue to cover macOS and Linux; Windows remains out of scope.

## Build and release flow

- Pull requests run the macOS compile check and release-script tests only.
- Release builds and packaging run once from the release tag workflow; do not duplicate a full release build in pull-request CI.
- A release still requires explicit user confirmation and must use the repository's real GitHub Actions and release state as evidence.
- When waiting for external CI or release jobs, stop after 10 minutes if they have not completed. Report the current job and URL instead of continuing to poll.

# Preferences

## Shell Commands
- Use `;` instead of `&&` when chaining commands
- Do NOT push to GitHub unless explicitly told to in the prompt
- Answer questions directly first (yes/no when asked), then do only what was asked — nothing more
- Do not take any next action proactively — wait for the next instruction

## Project Context
- App: CeeDee-Ripper v1.0.0 — done, fully functional
- Goal: publish to snapcraft.io store under account `odioski`
- Snap name: `ceedee-ripper`
- Repo: `odioski/CeeDee-Ripper`, branch: `master`
- Credentials file: `./snapcraft-creds` (not `/tmp/`)

## CI
- Workflow: `.github/workflows/build.yml`
- Runner: `ubuntu-24.04`
- Snap built with `snapcore/action-build@v1 --use-lxd`
- Published with `snapcore/action-publish@v1`
- Secret: `SNAPCRAFT_STORE_CREDENTIALS` set in repo secrets
- Node.js 24 forced via `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true`

## Known Issues
- Stuck upload in snapcraft.io review queue blocks CI publish step
  — fix: reject it at https://dashboard.snapcraft.io/snaps/ceedee-ripper/
- GStreamer plugins need `GST_PLUGIN_SYSTEM_PATH` and `GST_PLUGIN_SCANNER` set in snap environment

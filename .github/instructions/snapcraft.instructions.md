---
description: Use when editing snapcraft.yaml or fixing Snapcraft schema/build errors in this repository.
applyTo: "**/snapcraft.yaml"
---

# Snapcraft Editing Guardrails

When working on Snapcraft config in this repository:

- Validate key names, key locations, and stanza structure against official Snapcraft docs before proposing edits.
- Prefer schema-safe keys and avoid inventing fields that are not listed in the reference.
- For app-level desktop integrations, confirm the exact key form accepted by the active Snapcraft schema.
- Keep changes minimal and scoped to the reported error.
- After edits, run a quick schema/build validation command when possible (for example, `snapcraft pack` or `snapcraft expand-extensions`).

Authoritative references:

- https://documentation.ubuntu.com/snapcraft/stable/reference/project-file/snapcraft-yaml/
- https://documentation.ubuntu.com/snapcraft/stable/reference/project-file/snapcraft-yaml/#apps
- https://documentation.ubuntu.com/snapcraft/stable/reference/project-file/snapcraft-yaml/#extensions

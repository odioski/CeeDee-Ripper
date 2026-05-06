# Snap Store Review Notes

## Current packaging direction

- Keep `confinement: strict` as the default target.
- Build the Rust binary inside Snapcraft with the Rust plugin.
- Only move to `confinement: classic` if the remaining failure is verified to be unsupported optical-drive or raw device access under strict confinement.

## Why classic is not the default

- Snap Store guidance does not accept classic confinement just because strict packaging is difficult.
- The current repo work is aimed at removing packaging/runtime mismatches first so any remaining failure can be attributed to confinement rather than a broken snap payload.

## Evidence to collect before requesting classic

1. The strict snap starts cleanly without the previous GTK/GIO/GStreamer runtime mismatch.
2. The app still fails only when accessing the optical device.
3. The failure is captured from the installed snap with command output or logs.
4. Interface state is recorded with `snap connections ceedee-ripper`.
5. Any manual interface connection attempt has already been tried.

## Store request summary draft

If strict confinement still blocks raw optical access after the snap payload is fixed, use this summary as the basis for a store request:

"CeeDee Ripper is a desktop audio CD ripper. The snap was first corrected to build the application and runtime payload entirely inside Snapcraft under strict confinement. After that packaging fix, the remaining failure is direct access to optical-drive device nodes required for CD detection/ripping. This is the only unresolved blocker. We are requesting classic confinement only for that unsupported device-access requirement, not as a workaround for packaging issues." 

## Publication commands

```bash
snapcraft login
snapcraft register ceedee-ripper
snapcraft upload --release=stable ./ceedee-ripper_*.snap
snapcraft status ceedee-ripper
snapcraft revisions ceedee-ripper --arch amd64
```
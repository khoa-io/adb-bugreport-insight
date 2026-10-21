# AOSP crash candidates — matching runtime qualification

Date: 2026-10-19.
This supplements the [agent-led investigation](agent-led-aosp-crash-investigation.md) and tracks [TASK023](../../docs/verification/03-open-qualification.md#aosp-runtime).

## Environment and preservation

The installed Google automotive SDK image has the exact captured fingerprint: `google/sdk_gcar_arm64/emulator_car64_arm64:15/AAG6.260127.001/14784989:userdebug/dev-keys`.
Its matching source/symbol manifest is still unavailable.
An APFS clone of the existing automotive AVD runs headlessly as `emulator-5580`; snapshots are neither loaded nor saved.
All device changes affect that disposable runtime.
The original AVD's file sizes and modification times are recorded before testing and compared afterward.
Raw runtime logs, APKs, dumps and test-only signing material remain private under `tmp/task022/`.
Public AOSP platform test keys do not match the preinstalled overlay's signing certificate.

## Before and diagnostic intervention

| Condition | Observation interval | Fatal headers | PhoneWindow line-166 signatures | Interpretation |
| --- | --- | --- | --- | --- |
| Original enabled ultrawide framework overlay | 55.29 seconds | 230 | 230 | Exact-build reproduction of the original Java failure |
| Global ultrawide framework overlay disabled | 55.06 seconds | 0 | 0 | Configuration-dependent failure; cluster activity resumes |
| Original overlay restored | Short diagnostic interval | 61 | 60 | Crash activity returns; last stack may be incomplete at capture |
| Scoped overlay, live enable/relaunch | 55.08 seconds | 1 | 0 | Original signature absent; separate ClusterHome listener-registration failure during resource relaunch |
| Scoped overlay, settled | 55.07 seconds | 0 | 0 | Cluster stays running after resource transition |
| Scoped overlay, after OS reboot | 55.08 seconds | 0 | 0 | No fatal exceptions during startup log or measured interval |

The disabled-overlay run is a diagnostic intervention, not acceptance of the scoped resource patch: it removes the intended primary-panel cutout as well.
The cluster activity is resumed and visible in ActivityManager's state during that run.
The source-level path from oversized cutout state to a negative content-fitting argument remains untraced.
The real InsetsSource geometry check alone produces a positive inset, so it cannot establish that missing causal step.

## Actual-runtime callback regression

The device harness reflects the actual PhoneWindow default listener and uses real View, WindowInsets, Insets and Pair classes through app_process.
Seven negative-input cases fail on that baseline listener.
The local candidate callback passes nine fitting cases and 27 layout-flag cases.
This validates the containment contract on the captured runtime's classes.
It does not replace the installed framework listener or exercise actual ViewRoot dispatch with a patched framework.
The harness source is part of the external Java candidate bundle and is not retained in this repository.

## Deployment checks and limits

The scoped overlay compiles against the exact target framework-res.apk; its private mask-bool resource ID differs from the cached r17 image, so cached resource APKs must not be deployed directly.
An independently signed pre-Q data overlay is rejected by PackageManager because it requires a platform signature or target Q and later.
For runtime validation, the same candidate resources are linked against the exact target framework-res.apk and packaged under a separate test name.
A temporary writable system copy permits installing it under /product/overlay, where PackageManager accepts it and idmap resolves all seven intended overrides.
The enabled test overlay preserves the primary 1800-pixel right cutout and removes the cluster cutout in live DisplayManager state.
It is enabled for system user 0 and driver user 10.
Both panel screenshots were visually inspected: the primary home layout and cluster dashboard render.
The screenshots establish visible content, not complete layout acceptance.
The temporary system copy is discarded at emulator exit.

Live enabling triggers a separate `com.android.car.cluster.home` activity-relaunch failure: CarUserManager rejects duplicate listener registration from ClusterHomeApplication.initClusterHome.
This is distinct from the original osdouble negative-inset failure and must be reported rather than hidden by counting only the original signature.
The settled interval has no fatal exceptions.
After OS reboot, startup logs contain neither Java fatal exceptions nor the duplicate-listener signature, and a further 55.08-second observation has zero fatal exceptions.
Live resource-transition robustness remains separate from the demonstrated startup configuration result.

The native candidate retains deterministic regression and full Android ARM64 translation-unit compile evidence.
No real H4 delayed/duplicate response fault injection or supported historical cause elimination is claimed.
Repeated cold boots, rotation/resizing, screenshot review, repaired-device bugreport comparison and framework/HAL product builds remain qualification obligations.

## New bugreport inspected through MCP

A fresh adb bugreport from the corrected isolated runtime was opened with the real stdio MCP server.
The agent discovered the main report as entry 335, then chose complete literal scans for the original PhoneWindow signature, test overlay and newly observed listener error.
Each scan completed all 51,087,368 decoded bytes in two explicitly confirmed pages, with no skipped lines or entries.
The 12 PhoneWindow hits are embedded Dropbox records, rather than current logcat failures; a bounded read around lines 576620–576650 confirms the historical osdouble crash timestamp 07:16:01.426.
The overlay read at lines 444107–444129 confirms STATE_ENABLED for both users 0 and 10.
Two listener-error hits refer to the single resource-transition crash at 07:17:13.220, confirmed with a bounded read around lines 577208–577242.
Retained Dropbox history must not be treated as failure recurrence after correction.
The captured report and raw MCP responses remain private; these locators are specific to that new report, not the original reference archive.

## Review and preservation checks

The original AVD metadata remains unchanged after the isolated experiment.
The temporary emulator is shut down after evidence collection.
Candidate resources match the canonical display config byte for byte; only the runtime test package/manifest/signature differ.
Documentation navigation and patch-local link checks pass, and git diff --check is clean.
No Rust product source changed in this investigation; earlier 200-test workspace results are historical evidence, not a fresh test run for these external AOSP changes.

The later [persistent replacement record](persistent-fixed-avd.md) covers the user's subsequent request to create a lasting corrected AVD and delete the original.
The original-AVD preservation statements above describe the completed disposable experiment before that later authorized removal.

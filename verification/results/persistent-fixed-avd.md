# Persistent automotive AVD replacement

## Request and acceptance

The user requested deletion of the original crashing AVD and asked whether a corrected AVD had been created.
The earlier TASK022 AVD was a disposable test clone: its system correction was discarded when the emulator exited.
It was not a permanent corrected replacement.
This follow-up creates an independent persistent replacement, verifies stop/start persistence, and only then removes the original AVD.
The deployed correction is the scoped cluster display resource mapping; framework and Bluetooth HAL candidates remain uninstalled.

Acceptance requires the original captured build fingerprint, primary cutout preserved, cluster cutout absent, the resource overlay enabled after a complete emulator restart, and no original Java signature during a measured interval of at least 55 seconds.
Full native historical-crash qualification remains under [TASK023](../../docs/verification/03-open-qualification.md#aosp-runtime).

## Construction and independence

Replacement name: `Automotive_Ultrawide_Fixed`.
Its AVD directory is `~/.android/avd/Automotive_Ultrawide_Fixed.avd`.
The complete userdata and encryption-key disk chains are copied independently, preserving their original format and relative backing files within the replacement directory.
An initial raw-userdata conversion did not boot correctly; it was replaced with the complete original-format disk pair before provisioning.
The replacement has private APFS clones of the installed SDK image files, and its config points to that private image directory.
Encryption data and emulator topology/settings are copied into the replacement.
Old snapshots and generated hardware paths are not copied.
The shared SDK package remains installed and unchanged.

The existing verified candidate resource APK is installed into the replacement's system overlay partition and enabled for users 0 and 10.
Provisioning uses the emulator's writable-system mode and adb remount.
A first normal restart ignored the writable-system layer and did not contain the correction.
The replacement's system/vendor qcow2 changes are therefore flattened into its private raw image files, and stale layers are removed while the emulator is stopped.
The flattened image initially retained the APK in the scratch overlay filesystem; read-only idmap mapping failed even though the APK read back with the correct SHA-256.
The APK is therefore written into the private product ext4 filesystem with root ownership, mode 0644 and the system_file SELinux label.
The stale copy is removed from the private F2FS scratch upper directory.
Product ext4 verification passes after editing, and the scratch filesystem is cleanly unmounted before its extents are restored.
Only the replacement's image extents are updated; no shared SDK or original AVD image is modified.
A subsequent normal restart without writable-system mode is required for acceptance.
This is a corrected SDK AVD configuration, not a complete AOSP source rebuild or a native-crash repair claim.

## Verification results

A normal-mode launch after complete emulator shutdown finds the overlay in the private product filesystem.
PackageManager reports STATE_ENABLED for users 0 and 10, with a valid idmap.
DisplayManager confirms the primary cutout bounds remain `(2104, 0)-(3904, 1800)` and the cluster has no cutout.
Both screenshots were visually inspected: the primary home screen and cluster dashboard render.
The fingerprint remains `google/sdk_gcar_arm64/emulator_car64_arm64:15/AAG6.260127.001/14784989:userdebug/dev-keys`.
All userdata, encryption, system, vendor and cache backing chains resolve entirely inside the replacement directory; its config does not reference the original AVD.
The normal-startup log has zero Java fatal exceptions and zero original PhoneWindow signatures.
A subsequent 55.04-second observation also has zero Java fatal exceptions and zero original signatures.
After acceptance, the replacement was cleanly stopped and the Android CLI successfully removed `Automotive_Ultrawide`.
Both the original .avd directory and .ini registration are confirmed absent; the replacement directory and registration remain.
The disposable TASK022 clone and intermediate partition files were cleaned up after preserving the private evidence.
The shared SDK package, reference bugreport ZIP, AOSP checkouts and reviewable patch bundles are retained.
Private logs and build material stay under `tmp/task022/runtime/`.
The [earlier runtime record](aosp-runtime-qualification.md) remains evidence for the disposable test clone, rather than evidence that this persistent replacement survived restart.

## SWE traceability for this deployment

| Level | Activity |
| --- | --- |
| SWE.1 | Define persistent restart, display compatibility and safe original-replacement acceptance above |
| SWE.2 | Keep correction in product display resources; isolate replacement disk/image ownership from the original and shared SDK |
| SWE.3 | Deploy the previously reviewed resource payload without introducing framework/HAL changes |
| SWE.4 | Reuse the unchanged payload's four allocation and six actual-framework geometry checks |
| SWE.5 | Check PackageManager/idmap resolution, both users and backing-image independence |
| SWE.6 | Fully stop/restart without writable-system, inspect live geometry and a measured crash interval, then perform authorized original removal |

## Launch

The replacement is registered in the normal Android AVD directory and uses private image files.
A standard launch does not need writable-system mode or an overlay reinstall:

```sh
emulator -avd Automotive_Ultrawide_Fixed
```

Its private SDK-image copy is intentionally outside shared SDK package updates.
Rebase and requalify the correction explicitly when updating this replacement's Android image.

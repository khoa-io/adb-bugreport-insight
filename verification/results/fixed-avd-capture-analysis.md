# Fixed automotive AVD — fresh capture and MCP analysis

## Purpose and method

The user requested starting the permanent corrected AVD, capturing a fresh bugreport and screen video, and inspecting the new archive with bugreport-insight.
This continues [TASK023](../../docs/verification/03-open-qualification.md#aosp-runtime) using the [persistent replacement](persistent-fixed-avd.md).
The AVD is launched normally with its window visible, without writable-system mode or snapshots.
The primary and cluster displays are recorded separately with screenrecord's 60-second time limit during passive observation after boot completion.
The actual MP4 durations are 16.601 seconds for primary and 32.840 seconds for cluster, with sparse encoded frames; these files must not be described as continuous 60-second playback.
Both H.264 streams decode, and decoded contact frames were visually inspected.
The bugreport is collected after recording, so its collection follows the observed interval.
The actual bugreport-insight MCP stdio server provides archive discovery, bounded confirmed reads and paginated searches.
The investigator chooses follow-up queries from the returned evidence; complete scans and bounded excerpts retain their distinct coverage limits.
Raw archives, videos, logcat, MCP transcripts and analysis artifacts remain private under tmp/task023.
The capture exposed a bugreport-insight generation defect; its scoped Rust correction follows the full SWE process in the [missing-memory provenance record](missing-memory-provenance.md).
No AOSP source or installed framework/HAL/application binary was changed during this capture exercise.

## Results

The AVD remains running with its window visible as emulator-5582.
The captured ZIP is 12,461,363 bytes with 340 entries and passes complete ZIP CRC validation.
Its SHA-256 is `9866d6d43d7498cd21ca1838b85bc1e4c89ae977e71a3799afdd99498434c55f`.
Main report entry 337 contains 54,216,532 decoded bytes.
The fingerprint is the same captured Google automotive build, AAG6.260127.001/14784989.
The main report's retained log window spans 13:01:07.404–13:12:15.607, rather than a single instantaneous snapshot.

| Observation | Result | Evidence |
| --- | --- | --- |
| Original PhoneWindow negative-inset loop | No recurrence in current logcat; all 13 stack matches are earlier Dropbox history | Main lines 463289–463934; each record's timestamp was read through MCP, all 12:49:00–12:50:36 before this boot |
| Java fatal records | 23 distinct timestamp/PID/TID headers; no duplicate header presentations found | Complete main-entry scan and bounded context reads for every header |
| Current native fatal-signal headers | Seven: two carwatchdogd SIGSEGVs and five audioserver SIGABRTs | Main lines 42523, 469754, 480827, 487236, 494439, 497671, 505454 |
| SystemUI responsiveness | Five ActivityManager ANR reports plus one WindowManager announcement | Main lines 33856, 34728, 471904, 477785, 482782, 504975 |
| Archived tombstones | 50 text files plus 50 protobuf companions: one current watchdog crash, 46 historical Bluetooth opcode mismatches and three historical Controller timeouts | Glob discovery and complete 50-file header census; protobuf companions are not extra incidents |
| Generated artifacts | Seven files generated and all seven read back through MCP resources | Static generation passes after the provenance correction |

This is not a healthy-device acceptance result.
The prior zero-fatal results were bounded observations; they must not be generalized to a stable complete AVD.
The original display correction remains effective against its specific failure signature, while these newly reproduced failures require separate qualification.

## Current Java failure groups

| Process | Distinct fatal records | Main failures |
| --- | --- | --- |
| com.android.car.carlauncher | 7 | Null Car while obtaining a manager during activity creation |
| com.android.car | 5 | Four CarServiceHelper connection failures; one audio-routing policy registration failure |
| com.android.systemui | 5 | Two DeadSystemException records and null car-manager failures in volume, task monitoring and HVAC paths |
| com.android.car.cluster.home | 2 | Duplicate listener registration, then null ClusterState during initialization |
| com.android.car.media | 2 | Null Car/CarMediaManager during service creation |
| com.google.android.companiondevicesupport | 2 | DeadObjectException during service stop |

Several failures follow loss/unavailability of car-service dependencies; counting them as 23 independent root causes would be unsupported.
The static ANR artifact retains six records: its first two have the same PID and focus-timeout reason but different event/report timestamps.
There are five distinct ActivityManager report headers; independent episode identity for that paired representation is not established, so this record does not claim six independent ANRs.

## ClusterHome lifecycle mechanism

MCP evidence shows onActivityStopped resetting mIsInitialized at 13:02:05.739 (main line 35619).
At 13:02:24.828, the same process crashes when onActivityStarted calls initClusterHome, which calls CarUserManager.addListener for an already registered listener (main lines 39929–39957).
The installed ClusterHomeSample APK was pulled separately as supplementary source evidence, and its actual DEX confirms that stop/destroy callbacks reset the flag while the start callback invokes initialization again.
The initialization method checks that flag, registers the user listener and sets it true, while the stop/destroy callback does not unregister the existing listener.
This supports a lifecycle-state/registration mismatch, rather than a renewed negative-inset failure.

The local Car/Cluster source revision `056ffd93c715a36187cea96b202c0a600da51673` differs: it guards initialization in onActivityPreCreated and has an empty onActivityStarted callback.
Its source must not be treated as the exact installed implementation.
The repair direction is to make process-scoped initialization and listener ownership consistent across stop/start and recreation, with explicit partial-initialization and car-service-reconnect tests; merely catching the duplicate-listener exception would not repair ownership.

## Native failures and causality limits

The current text tombstone is entry 206, FS/data/tombstones/tombstone_30.
Its lines 5–13 identify the current carwatchdogd null dereference; lines 26–29 place it in WatchdogProcessService.updateVhal, called by reportWatchdogAliveToVhal.
The main report has a second carwatchdogd SIGSEGV later in collection.
The five audioserver fatal-signal headers correlate with explicit TimeCheck timeouts for IAudioPolicyService.registerPolicyMixes; the first is recorded at main lines 469724–469754.
Requested audio-HAL thread dumps and DEBUG Cmdline headers are not automatically additional fatal incidents.

The 49 retained Bluetooth tombstones are dated May 20, months before this run; they do not establish current Bluetooth recurrence or elimination of the original historical cause.
The first current watchdog tombstone replaces one old mismatch slot, explaining 46 retained mismatches here versus 47 in the original reference archive.
The earlier Bluetooth reset-timeout candidate is not a demonstrated repair for these watchdog or audio crashes.

Local watchdog source inspection identifies unguarded VHAL/client-value dereference boundaries after connection/copying the client, but the exact faulting operand and concurrency sequence require matching symbols and a controlled reproduction.
Trace VHAL disconnect/reconnect ownership and the blocking audio-policy/HAL call chain before changing watchdog timeouts.
The failures occurred during a cold-boot/collection run; temporal overlap with capture does not prove the recorder, collection load or display overlay caused them.
An unchanged matching SDK image under the same launch/recording workload is the necessary control for that attribution.

## Tool failure corrected and verification

The initial static CLI run failed with missing memory_snapshot evidence_origin because SERVICE meminfo existed without recognized summary data.
The correction uses the existing section-header provenance as a fallback while leaving unknown memory measurements explicit.
It does not relax validation or invent RAM values.
The baseline fails and the corrected real report publishes all seven artifacts; generated source references were read back through real MCP resources.
The synthetic regressions, full 202-test workspace run, offline build, formatting and warnings-denied Clippy pass.
The [SWE record](missing-memory-provenance.md) links the requirement, design and verification changes.

All selected main-entry grep scans exhaust the full 54,216,532 bytes without skipped lines or entries; individual read excerpts retain their bounded scope.
A literal path query for tombstone was insufficient for subtree discovery; the corrected glob query discovered the full 100-file set and led to the header census.
Producer-side DUMPSYS/service timeouts remain evidence gaps even when the ZIP is valid and the MCP scan is complete.
The local adb client exceeded 180 seconds while Android continued collecting; the completed device archive was recovered after dumpstate finished and then CRC-validated.
Archive SHA-256 and nanosecond mtime remain unchanged after generation and MCP inspection.

Private evidence directory: `tmp/task023/capture-20261019-130101/`.
It contains the original primary/cluster MP4s, video metadata and decoded contact frames, fresh ZIP, generated insight set, MCP request/response records, tombstone census, per-fatal context reads and supplementary APK/DEX evidence.
No staging, commit or remote publication is performed.

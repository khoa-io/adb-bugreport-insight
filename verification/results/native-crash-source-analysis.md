# Native Bluetooth crash source investigation — 2026-10-19

This is the native-crash workstream of [TASK022](agent-led-aosp-crash-investigation.md).
The user authorized an adaptive MCP bugreport investigation, AOSP source inspection through Lima, and reviewable fixes through SWE.1–SWE.6.
The historical incidents are identified; their root cause is not yet demonstrated.
The source hardening below is a candidate with a passing focused regression, not a verified fix for the captured incidents.

## Evidence and causal limits

The AI investigator used actual `bugreport-insight` MCP calls, with explicitly confirmed content reads, to enumerate and read all 50 textual tombstones.
Binary protobuf counterparts were not counted as additional incidents.
The local evidence ledger is `tmp/task022/native-census.json`; the parent investigation retains the MCP transcript.

| Group | Incidents | Evidence | Interpretation |
| --- | --- | --- | --- |
| HCI response mismatch | 47 | `hci_layer.cc:214`, waiting for `SET_EVENT_MASK (0x0c01)`, received `RESET (0x0c03)` | The response opcode does not match the outstanding stack command; the invariant correctly refuses to consume it as a Set Event Mask response |
| Controller startup deadline | 3 | Entries 305, 324 and 350; `stack_manager.cc:57`, last instance `starting Controller` | Controller startup did not complete by the stack deadline; the records do not establish why |

All 50 records concern Bluetooth and date from May 20, 2026, between 02:35:41 and 05:10:55 at UTC+07:00.
They are historical relative to the September 24 capture.
For example, entry 265 (`tombstone_29`) identifies `com.google.android.bluetooth`, uptime 0 seconds, thread `bt_stack_manage`, and the HCI mismatch abort.
Entry 305 (`tombstone_09`) identifies the Controller startup abort.
The observed `libbluetooth_jni` Build ID is `2a1aff22665aba884024f7493b05580f`.

The current main report establishes the AIDL default HAL implementation and records a successful reset during September 24 startup: main entry 395, lines 28433–28444, around 00:27:01.965–01.970 at UTC+07:00.
That reset completed in approximately 4 ms.
It does not reproduce the historical aborts or show a reset timeout on that boot.
The main captured build `AAG6.260127.001` is not an exact match for the locally available Android 15 tag.

## Source inspection and alternatives

Lima MCP source inspection used the existing Android tree, manifest revision `refs/tags/android-15.0.0_r17`.
The original checkouts were left clean.

| Project revision | Unit | Source finding |
| --- | --- | --- |
| Bluetooth `6b03a6030590895bf12f92315ad5add54a644431` | `system/gd/hci/hci_layer.cc`, `handle_command_response`, lines 194–215 | Command completion must correspond to the current outstanding opcode; no safe basis exists for silently consuming a mismatched reset |
| Same | `system/gd/stack_manager.cc`, `StartUp`, lines 50–58 | Startup waits for a configured deadline then fails if the controller has not completed startup |
| hardware/interfaces `b04e2f3df5ebbbeea46f555d0965357f05aa1457` | `bluetooth/aidl/default/BluetoothHci.cpp`, `reset`, lines 135–198 | HAL-owned Reset waits one second, logs timeout, then returns no success/failure indication |
| Same | `BluetoothHci::initialize`, lines 243–298 | Goldfish/Cuttlefish reset is followed by stack packet callbacks and initialization SUCCESS even if reset timed out |

A delayed HAL-owned reset completion can reach a newly initialized stack after this unconditional handoff.
This is a concrete source defect and a plausible path to the observed opcode mismatch, but the tombstones alone do not prove that path happened.
A stale response that incorrectly satisfies the HAL's own reset, a duplicated controller response, or another controller command source can produce a similar mismatch.
HCI reset completion has no transaction ID, so accepting the first matching opcode does not prove transport synchronization.
Ignoring extra Reset completions in the stack would conceal that uncertainty and risk corrupting command sequencing.
Increasing startup timeouts without a measured slow startup would similarly hide a symptom rather than establish a correction.

## SWE.1 — Candidate obligation and acceptance

This external AOSP change does not alter any `bugreport-insight` requirement or MCP contract.
Its local obligation NATIVE-RESET-1 is: on emulator HAL initialization, failure to receive a successful HAL-owned reset completion within the existing deadline must prevent SUCCESS and prevent handing that transport to stack-owned command processing.
The failed attempt must stop watching the transport, close it, discard failed-attempt parser/callback state, and restore READY so a subsequent attempt can acquire a new transport.
A valid reset retains normal initialization behavior.
The HCI command-response invariant and stack startup deadline remain enforced.

## SWE.2 — Allocation and design decision

Allocate NATIVE-RESET-1 to the AIDL default Bluetooth HAL's transport initialization boundary.
Keep the HAL responsible for its own preparatory reset; keep the Bluetooth stack responsible for correlating its own commands.
The candidate does not change Rootcanal, the controller command protocol, or stack recovery policy.
Failing this initialization can leave Bluetooth unavailable until a later retry; that is preferable to explicitly declaring an unsynchronized stream ready.
Closing a stream is not a demonstrated controller epoch reset, and a later reconnect may still see controller-level stale state.

## SWE.3 — Construction

The external patch and test bundle contains `reset-timeout.patch` and `verify_reset_contract.py`; it is not retained in this repository.
Reviewable local source snapshots are in `tmp/task022/native/`:

- `base/BluetoothHci.cpp` and `.h`: exact source snapshots from the stated hardware/interfaces revision.
- `candidate/BluetoothHci.cpp` and `.h`: candidate source.
- `reset-timeout.patch`: a patch rooted at the hardware/interfaces project.
- `verify_reset_contract.py`: focused compiled regression harness.

`BluetoothHci::reset` returns true only when sending succeeds and the completion future becomes ready within the existing one-second timeout.
On false, `initialize` closes the Linux HCI or serial transport, clears `mFd`, `mH4` and `mCb`, restores READY, and reports `HARDWARE_INITIALIZATION_ERROR` through the existing callback.
Death-recipient linkage occurs after the preparatory reset so a failed attempt does not install a new recipient.
The candidate does not suppress either native abort.

## SWE.4 — Executed unit contract checks

The harness extracts the actual `reset` and `initialize` methods from the selected source file and compiles them as C++17 using the host `clang++` with `-Wall -Wextra -Werror`.
Binder, fd watcher, Linux HCI management and H4 parser boundaries are deterministic doubles.
The one-second production timeout is retained.
The descriptor close call is intercepted so the test cannot close unrelated host descriptors.

| Command from repository root | Expected | Actual |
| --- | --- | --- |
| `python3 tmp/task022/native/verify_reset_contract.py tmp/task022/native/base/BluetoothHci.cpp` | Baseline violates NATIVE-RESET-1 | Exit 1: timeout reported SUCCESS to stack |
| `python3 tmp/task022/native/verify_reset_contract.py tmp/task022/native/candidate/BluetoothHci.cpp` | Candidate refuses handoff, disposes state/transport, allows a later successful reset | Exit 0; all assertions passed |

Assertions cover the error callback, stopped watcher, closed descriptor, cleared parser/callback/descriptor, READY state, absence of failed-attempt death linkage, and successful retry with normal callback/watch state.
These checks demonstrate the focused contract with doubles, not real Android Binder or H4 concurrency.

## SWE.5 — Integration evidence and gaps

Lima `git apply --check /tmp/task022-native-reset-timeout.patch` in hardware/interfaces passed at the recorded revision.
No original AOSP source file was modified to obtain that result.
The full patched `BluetoothHci.cpp` translation unit was also compiled successfully with the actual cached Android arm64 Soong command, Clang `clang-r530567`, `-std=gnu++20`, generated Binder headers, `-Werror`, and ThinLTO.
Lima ran the existing `aosp-build:latest` image as `linux/amd64` with source/output read-only and candidate `.cpp`/`.h` file overlays.
Only `-o` and `-MF` were redirected to private `/tmp/task022-native/` output.
Exit status was 0 with no diagnostics; object LLVM bitcode SHA-256 was `d06909933066b1874ff21fda076861ef5ae5372ceda2be51e53fd3280fa8919e`.
The external bundle README records command derivation and build scope.
The original hardware/interfaces checkout remained clean after compilation.
This compile establishes full translation-unit compatibility with the cached Android headers and flags, not full service linking or real transport integration.
Before accepting the change, link the default HAL service in an isolated checkout and exercise initialization with delayed, missing, malformed and duplicated Reset responses over the real H4 transport.
Exercise both managed Linux HCI and serial fallback paths and validate failed initialization, reinitialization, close, and client death.
Verify there is no false SUCCESS and no callback from a failed-attempt watcher after disposal.

## SWE.6 — Incident verification and disposition

NATIVE-RESET-1 has focused regression evidence; historical incident resolution remains **not demonstrated**.
Neither the 47 HCI mismatches nor the three Controller startup timeouts are declared fixed.
The candidate is not installed on the captured system.
No full image, emulator reproduction or repaired-device bugreport was produced by this workstream.

For qualification, obtain source/symbol provenance matching the captured Build ID and a reproducer retaining HCI snoop plus HAL/controller logs across reconnects.
Correlate each outgoing Reset and Set Event Mask with each incoming completion to distinguish timeout handoff, stale response, duplicate response and controller reset sources.
Reproduce startup under the measured workload, then compare baseline and candidate with the same injected faults and repeated Bluetooth restarts.
Treat the three startup-deadline failures separately until their blocking operation is established.
The September 24 successful reset is useful negative evidence against claiming a current repeated native failure, and must remain visible in the conclusion.

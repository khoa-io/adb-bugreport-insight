# Agent-led bugreport analysis and AOSP fix candidates

## Outcome and scope

Date: 2026-10-19.
TASK022 implements the user's intended workflow: an AI investigator chooses MCP queries, interprets returned evidence, inspects relevant source through Lima and prepares tested fixes.
This extends TASK020's scripted acceptance exercise with adaptive investigation and source-level reasoning.
The archive remains read-only; source checkouts remain clean, and reviewable external AOSP patches are supplied separately.
No changes to the Rust executable, MCP tool set or promoted analyzer scope are required.
The patches have focused verification; runtime experiments use an isolated clone of the exact captured SDK build, with candidate acceptance distinguished from diagnostic interventions.

## Investigation environment and evidence trail

Tool baseline is `b31987d7d3e3e77452ac322d4a35b6f93ff36d64` plus the existing uncommitted implementation and TASK021 lifecycle correction.
The input is the existing private reference ZIP, 14,710,126 stored bytes, 398 entries; its main entry is ID 395 with 64,645,191 decoded bytes.
Requests use the actual shipped executable's stdio MCP surface, including initialization and explicit confirmation of content work.
The investigator first reads metadata/path discovery, then chooses bounded entry/range/search requests and follows returned source locators.
Each search continuation is separately confirmed; completed scans below exhaust the selected main entry without skipped lines or entries.
Read output is bounded per excerpt; clipped display-dump lines are treated as clipped evidence rather than assumed complete.
Only parsed MCP responses are used as primary archive evidence; there is no direct ZIP extraction used to replace MCP inspection.
The earlier generated artifacts are secondary corroboration, not the source of the causal claims.

The private ledger under `tmp/task022/` retains the raw console request/response transcript and subsequent MCP result pages, tombstone census, window/overlay context and duplicate-header review.
It remains ignored because it contains real report content.
Shared records retain entry IDs, locators, aggregate facts, public platform component names and source references rather than committing report text.

Lima guest status and mounts were checked before inspecting the source.
The historical development container is absent, but the source and output volumes and cached amd64 builder are available.
Short-lived validation containers use read-only source/output mounts and private test outputs.
The existing manifest selects `refs/tags/android-15.0.0_r17`; all required source projects are already present, so no repo sync or source download is necessary.
The captured Google automotive build is not the local phone-emulator build; an Android 15 tag and matching line locations do not establish exact source provenance.

## Adaptive query and decision trace

| Question / selected MCP operation | Returned evidence | Investigator's next decision |
| --- | --- | --- |
| find_entries for text files and tombstone subtree | Main report ID 395; small Java Dropbox reports; 50 text tombstones plus protobuf counterparts | Read a complete small Java crash report and representative native headers before scanning large content |
| read_entry 129, lines 1–55 | Complete 45-line Java crash report; source uptime and capture timestamp; PhoneWindow → WindowInsets.inset → nonnegative precondition | Inspect those exact framework methods and the cluster app; identify which argument contract is violated |
| read_entry 265 and 305, first 45 lines | HCI opcode mismatch and Controller startup timeout; May timestamps | Census all 50 textual tombstones, group by abort signature, and compare with the current startup logs |
| Read first 13 lines of each textual tombstone through MCP | Exactly 47 mismatch aborts and 3 startup-deadline aborts, all May 20 | Treat these as two historical groups, not 50 unrelated current defects |
| grep_archive for InsetsState, captionBar and cluster activity | Small cluster display, cutout geometry and repeated activity failures | Read the relevant display/window ranges; inspect resource fallback and source arithmetic |
| read_entry 395 around 1588–1595, 8031–8038 and 326659–326665 | An 1800-pixel right cutout appears on both 3904-pixel primary and 528-pixel cluster panels; the latter bounds extend left of zero | Trace the automotive ultrawide resource, then test whether this geometry actually produces a negative inset |
| grep unique IDs / overlay names; read 518478–518490 | Two internal panels; automotive ultrawide framework overlay explicitly enabled | Prepare a product-specific cluster mapping rather than globally changing framework fallback semantics |
| grep current Bluetooth HAL, then read 28433–28444 | AIDL default HAL; current Reset succeeds in about 4 ms and initialization completes | Reject a claim of ongoing native crash reproduction; inspect the historical command-sequencing failure and HAL timeout boundary separately |
| Complete PhoneWindow, fatal-header and manager-kill scans | Repeated current Java signature; duplicated records from different report buffers | Independently reconcile raw counts with retained generated crash records before reporting incident totals |

The model selected follow-up queries after reviewing intermediate results.

## Java crash diagnosis

The current repeated failure is in `com.android.car.cluster.osdouble` during default window-inset fitting.
The crash stack reaches the nonnegative-argument check through PhoneWindow's default callback, which passes legacy inset amounts directly to WindowInsets.inset.
This proves a negative amount at that boundary; local line 1155 identifies the left check, conditional on source/debug-line correspondence.
The exact crash loop is now reproduced on the matching SDK runtime; the internal producer of the negative amount remains untraced.
It must not be inferred merely from the negative coordinate of a cutout rectangle.

The full main-report fatal-header scan returns 274 lines.
Matching timestamp, PID, TID and fatal message independently identifies 271 distinct records and three duplicate pairs: lines 616295/660959, 616557/681121 and 616678/681242.
Some repeated buffer presentations omit UID while others include it; normalization must account for both formats.
The 273 raw manager-kill messages similarly reduce to 270 distinct records across the same repeated buffer presentations.
This corroborates the generated artifact's 271 retained Java records and explains its three suppressed raw hits without labelling those duplicate presentations as extra crashes.
The first and last repeated stack occurrences in the main report span about 50.2 seconds.

The Java candidate clamps negative amounts only at the default content-fitting callback; its external patch and regression bundle are not retained in this repository.
Positive sides, the layout-flag bypass and existing inset consumption remain unchanged.
[The source analysis](java-crash-source-analysis.md) records the alternatives and failure boundaries.
The actual-class callback check reproduces seven failures before the change; afterward all nine fitting inputs and 27 layout-flag cases pass.
This proves containment of that argument failure for the tested callback, not elimination of every upstream geometry or visual-layout defect.

## Display configuration diagnosis

The enabled automotive ultrawide overlay's 1200-dp right-anchored radius at density 1.5 predicts the observed 1800-pixel bounds on both panels exactly.
The global resource is inherited by the second internal display when no per-display mapping exists.
Applying the intended primary-panel cutout to the small rectangular cluster creates an oversized all-covering inset region.
This is a concrete product-configuration problem and a separate correction target.

An actual Android InsetsSource geometry check shows the oversized cluster source produces positive 528-pixel LEFT insets, while the primary produces positive 1800-pixel RIGHT insets.
It does not reproduce the negative-left exception, so the configuration correction and PhoneWindow containment must retain separate claims.
The configuration patch is scoped to the observed emulator topology and must not be applied blindly to products with different display IDs or intended cutouts.
The display candidate maps only the observed cluster ID at index zero, with explicitly empty cutout strings and fill disabled; primary and unknown displays retain their fallback. Its external patch bundle is not retained in this repository.
Using index zero also preserves the inherited main/default values of unrelated per-display arrays; a new primary/cluster two-index allocation would activate secondary defaults that may differ.
Four allocation checks, six real-framework geometry cases, exact patch application and complete AAPT2 resource compile/link pass.
The compiled resource dump confirms that both cluster strings are actually empty, rather than literal quote characters or absent array entries.

## Native crash diagnosis

All 50 textual tombstones are historical Bluetooth incidents from May 20, approximately four months before capture.
The 47 opcode mismatches show a Reset completion arriving while the stack expects Set Event Mask.
The three other aborts show Controller startup missing its deadline; they do not identify the blocking operation.
Complete searches of the current main report find neither historical abort signature, and current HAL startup completes successfully.

The available HAL source nevertheless contains a concrete defect: preparatory emulator Reset can time out and initialization still reports SUCCESS before handing the stream to the stack.
The native candidate prevents that handoff, reports the existing initialization error, disposes failed state and allows retry; its external patch bundle is not retained in this repository.
It preserves the stack's command-response invariant rather than concealing a mismatched response.
[The native source record](native-crash-source-analysis.md) retains competing causes: delayed/stale/duplicate controller responses and startup timing need matching-build transport evidence.
Its actual-method regression fails on the baseline and passes on the candidate.
The full patched Android ARM64 translation unit also compiles with real generated Binder headers and the cached Soong command.
Neither result proves the cause or resolution of the 47 historical mismatches or the three startup timeouts.

## SWE process and traceability

| Level | Contract/allocation and performed activity | Evidence / limit |
| --- | --- | --- |
| SWE.1 | REQ-MCP-03/06/09/12 support read-only, bounded, attributable, confirmed investigation; external candidate obligations define failure and compatibility acceptance separately | No new Rust product requirement or draft analyzer tool promoted; candidate criteria reside in the detailed workstream reports |
| SWE.2 | Existing ARCH-ARCHIVE/ARCH-MCP-SRV/ARCH-MCP-TOOLS-ARC boundaries remain; external allocations are PhoneWindow fitting policy, product display resources and HAL-owned reset synchronization | App failure, geometry producer and HAL/stack boundary are distinguished |
| SWE.3 | Prepare minimal revision-pinned AOSP patches, synthetic regression drivers and target tests without changing clean source checkouts | Patch bundles include application instructions and reproduction commands |
| SWE.4 | Reproduce Java negative-argument behavior with actual Android classes; test exact native reset/initialize methods with deterministic boundaries; inspect/test display lookup and geometry | Stubbed transport/Binder boundaries and synthetic View are explicitly identified |
| SWE.5 | Check patch application; compile the native Android translation unit with its actual generated headers/flags; attempt full Java class compilation | Native TU passes; complete Java standalone compile lacks the actual build classpath; framework instrumentation and full service linking remain unrun |
| SWE.6 | Compare investigation evidence, scoped before/after checks, configuration compatibility and current/historical distinctions | [TASK023](../../docs/verification/03-open-qualification.md#aosp-runtime) retains matching automotive build, live display dispatch, H4 fault injection and repaired-device verification |

## Review bundle and verification boundary

The separately maintained AOSP patch index provides the reviewable candidates and their source baselines; those patch bundles are outside this repository.
Changes are left uncommitted for user review; there is no upstream submission.
An installed SDK automotive image has the exact captured fingerprint, `google/sdk_gcar_arm64/emulator_car64_arm64:15/AAG6.260127.001/14784989:userdebug/dev-keys`.
Runtime checks use a private APFS clone of its AVD, serial `emulator-5580`, without loading or saving snapshots.
The original AVD metadata is checked for preservation.
The exact Google source manifest remains unavailable; the existing r17 source is still a comparison baseline.
Do not substitute a successful unit harness, a single translation-unit compile or a clean historical startup for a repaired-device result.

Runtime results and their deployment/acceptance limits are retained in the [runtime qualification record](aosp-runtime-qualification.md).

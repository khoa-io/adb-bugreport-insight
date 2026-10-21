# Java crash source analysis and candidate verification — 2026-10-19

## Finding and confidence

The reference archive inspected through `bugreport-insight` MCP contains a repeated Java exception in `com.android.car.cluster.osdouble`.
The decoded stack follows `PhoneWindow.lambda$static$0` → `WindowInsets.inset` → `Preconditions.checkArgumentNonnegative`.
This establishes a negative argument at the inset boundary.
Matching local source line 1155 is the left-argument check; identifying the side is conditional on source/debug-line alignment.

The checked framework revision is `d41da232c75a51b4a9023d8d418d7c0a5120c0e2` from the existing Android 15 r17 checkout.
Captured build `AAG6.260127.001/14784989` is a different build.
The method and line locations align at `PhoneWindow.java:166`, `WindowInsets.java:1128/1155`, and `View.java:13421`; this does not establish an exact source-manifest match.

The default callback obtains legacy system-window insets and passes them unchanged to `WindowInsets.inset(Insets)`, whose contract requires every amount to be nonnegative. `WindowInsets.getInsets` can propagate a negative component from a type map because aggregation starts with the first non-null entry.
Negative amounts therefore reach a checked operation and terminate the UI thread.
This failure mechanism is reproduced below using actual Android inset classes.

The dump also contains an oversized cutout on a small cluster display.
That is a separate configuration lead.
The normal `InsetsSource` intersection path clips that geometry; oversized cutout bounds alone do not prove the negative producer.
No patch here claims to correct that producer or the captured device configuration.
The captured numeric negative value is unknown; synthetic `-1272` exercises a geometry-sized value and is not reported as an observed inset.

## SWE.1–SWE.3: scoped correction

This external AOSP candidate implements the user's requested diagnosis-to-fix example; it does not change the Rust tool's committed software requirements.
The local acceptance criteria are scoped to the existing framework fitting policy:

| Concern | Criterion and allocation |
| --- | --- |
| Inputs | Default content-fitting callback receives legacy system insets with arbitrary signed components |
| Output | Fitting amounts equal componentwise max(input, zero); every valid positive side is retained |
| Failure | Negative fitting amounts do not cause the nonnegative-argument exception |
| Compatibility | Zero/positive inputs, layout-flag bypass, visibility, input immutability, and existing consumption remain unchanged |
| Architecture | Existing PhoneWindow policy → WindowInsets fitting/consumption; no new component or public API |
| Detailed unit | `sDefaultContentInsetsApplier` clamps only its fitted amounts using `Insets.max(raw, Insets.NONE)` before invoking `inset` |
| Alternatives | A blanket exception catch would conceal unrelated failures; changing public WindowInsets validation would relax all callers' contracts; disabling fitting would alter valid layouts |
| Risk | Framework behavior on invalid input changes from a crash to zero consumption on that side; invalid producer and visual correctness still require target investigation |

The external candidate bundle provides the patch, before/after excerpts, regression tests, and reproduction commands; it is not retained in this repository.
The source checkout was left clean; the candidate was applied only to a temporary snapshot for exploratory compilation.

## SWE.4: executed behavioral checks

The external callback harness compiles the extracted before/after Java callbacks.
It uses existing compiled Android `WindowInsets`, `Insets`, and `Pair` classes, including the actual negative-argument validation and consumption implementation.
Only `View` and the callback interface are synthetic; the test does not implement inset arithmetic.

Environment: Lima Linux guest, detached `aosp-build:latest` amd64 container, AOSP bundled JDK 21; source and output mounts read-only.
Existing class artifacts were built previously and are identified by SHA-256; they are not asserted to be a fresh build from the compared source revision:

| Artifact | SHA-256 |
| --- | --- |
| `framework-minus-apex/android_common/combined/framework.jar` | `d2dea46696269c0769121208391b473eb7f7f78b41d6f179c90d4ecb485d6892` |
| `framework-graphics.impl/android_common/javac/framework-graphics.jar` | `d984b96a71ec1a6aafe3e6c36add24354ad2629d790e6d7d935ddffb29e640b7` |

| Check | Expected | Actual verdict |
| --- | --- | --- |
| Baseline callback with 7 negative inputs | Actual WindowInsets validation throws | PASS: 7 exceptions |
| Candidate callback, zero and positive inputs | All sides unchanged | PASS: 2 inputs |
| Candidate callback, each negative side, mixed negatives, all negative, integer minimum | Clamp invalid sides, preserve valid sides, no exception | PASS: 7 inputs |
| Consumption, visibility, immutable input | Existing consumption retained; flags and input unchanged | PASS: all 9 inputs |
| Each of 3 layout flags across all 9 inputs | Insets.NONE and identical input object returned | PASS: 27 cases |
| Final patch applicability and whitespace | Applies cleanly to pinned baseline | PASS: read-only `git apply --check --whitespace=error` |
| Source checkout | No tracked/untracked changes caused by candidate | PASS: clean `git status --short` |

Successful check output:

```text
PASS: actual WindowInsets/Insets/Pair + extracted callbacks; 9 fitting inputs, 27 layout-flag cases; baseline 7 negative failures, patched 0.
LIMIT: synthetic View and callback interface; no Android ViewRoot or instrumentation execution.
```

The detached callback container exited zero.
The authored but unexecuted instrumentation tests exercise the actual private callback through reflection and cover the same cases; these external test sources are not retained in this repository.

## SWE.5–SWE.6: remaining evidence

| Activity | Verdict and concrete reason |
| --- | --- |
| Isolated full PhoneWindow compilation | NOT PASSED: exploratory javac attempts could not reproduce the full Android bootclasspath/generated annotation configuration; first lacked annotations/window flags, second hit java.base package overlap, third lacked compatibility annotations/XML parser |
| Framework instrumentation suite | NOT RUN: requires a configured framework build and connected compatible Android target |
| Complete target build | NOT RUN: the captured build's exact manifest is unavailable; existing phone-emulator output does not qualify the automotive build |
| ViewRoot/default callback integration | NOT VERIFIED: synthetic View bypasses attachment and real inset dispatch |
| Original automotive crash loop reproduction | PASS: isolated exact-build SDK image reproduces 230 fatal records and 230 PhoneWindow line-166 signatures in 55.29 seconds; see the [runtime record](aosp-runtime-qualification.md) |
| Software acceptance | NOT VERIFIED on device: survival and visual fitting across cold boot, rotation/resizing, and display configuration remain required |

The failed standalone compilation attempts are environment/classpath gaps, not evidence that the patch compiles or fails under the actual Android build.
The successful callback harness is meaningful unit evidence and explicitly does not replace these omitted integration and software checks.

The candidate is ready for source review and target validation.
It should not be reported as a deployed fix, a source-producer fix, or a verified elimination of all Java crashes in this report.

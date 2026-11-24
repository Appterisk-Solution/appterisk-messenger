# Implementation status — 0.2.0

> Current work: 0.3.0 desktop features are implemented and built; see [release verification](RELEASE_0.3.0.md) and the updated [architecture](ARCHITECTURE.md). Automated suites and NSIS filesystem-preservation checks passed. The 0.3.0 real-message/media/authentication matrix remains separate from the historical 0.2.0 results below.

The prerequisite passed before implementation: after being asked specifically about three distinct accounts through three full application restarts, the user replied, “tested 3 account and no issue”. This confirms the 0.1.2 baseline; it is not a new-version authentication claim.

Implemented: production sidebar shell; native single-account content area; dynamic account add/rename/switch/reorder/reload/remove; explicit existing-profile adoption; Rust SQLite schema/migrations and metadata; persistent UUID/profile mappings; startup/last-selection preferences; light/dark/system shell themes; saved language/notification preferences; native WebView reuse/hiding/resizing/dialog blocking; instance/profile locks; preserved browser-path fix and graceful browser exit; least-privilege command capabilities; restricted navigation/system-browser popups; optional original diagnostic probe.

Automated acceptance covers frontend behaviors, SQLite and account invariants, three-profile synthetic storage persistence, actual remote-origin IPC rejection, real-controller switching/reuse/visibility/resize/removal, normal shutdown and three metadata restart cycles. See [release evidence and remaining manual checks](RELEASE_0.2.0.md).

The user separately confirmed all three adopted accounts remained logged in through three restarts in the installed 0.2.0 manager. This is real-user authentication evidence, separate from installer filesystem checks. Drag/reload/removal and monitor/DPI behavior have automated coverage where documented, without additional user-confirmed manual acceptance. No notification, background-message, unread-count, media/call/download or non-Windows compatibility claims are made. Language preferences are saved; full shell localization is deferred.

The original plan is retained in LEGACY_IMPLEMENTATION_PLAN.md. Later phases and an external backend remain outside scope.

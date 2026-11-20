# Phase 4.2.1 visual review

The supplied 1672×941 Neon Spatial × Cyber-Tech concept was inspected directly from the user attachment. The comparison started with all thirteen Phase 4.2 Wayland captures and a fresh baseline run from the verified checkpoint. The artwork shown here is bundled original art. All before and after UI captures came from real Qt Quick windows under private KWin Wayland.

| Captures | Observed change and remaining difference from the concept |
| --- | --- |
| Worlds at 1920×1080 and 1440×900 | The hero sits on emitted orbital bands against a bright violet/cyan horizon. Adjacent cards recede with tilt, scale, depth and opacity. The left rail and lower Workspace previews follow the reference composition. The hero stays centered as selection changes. The deck deliberately ends at the first and last Worlds. |
| Selected World | The emerald river, green border and readable caption make the middle selection the visual anchor. The rendered name and counts come from the demo hierarchy. The selected research World has two Workspace previews because the demo contains two. |
| Intermediate and compact Worlds | At 1024×768 and 800×600 the rail gives way to header navigation and the deck shows fewer neighbors. At 630×470 only the hero remains. Paging, search, status and opening the World fit on screen. |
| Workspaces and selected Workspace | The illustrated World masthead and denser project tiles carry the hierarchy context into a distinct level. The active project has a marked border and a lower details action. The reference illustrates Workspace previews on Worlds, so the deeper matrix extends that language to the full cards. |
| Logical Desktops and window cards | Desktop selector cards establish context above a separate application shelf. The title and status stay readable. Window cards use identity initials and labeled observation metadata rather than screenshots of other applications. |
| Search and Control Center | Both use the same dark glass surfaces and rim treatment as the Worlds panel. Search keeps a stable frame while users type. Control Center uses truthful channel status and a visible setting focus ring. |
| Empty SQLite and observer unavailable | Empty SQLite shows a clear no-Worlds message. With a connected private controller and observer unavailable, a real SQLite-backed hierarchy still renders and the observer remains marked unavailable. |
| Reduced motion, low effects, high contrast | Reduced motion preserves layout and makes navigation immediate. Low effects removes art and decorative layers. High contrast keeps strong native focus and card boundaries. |

Baseline and current captures are in `artifacts/phase-4-2-1/before/` and `artifacts/phase-4-2-1/screenshots/`. The wide shell keeps the application's existing live breadcrumbs and source indicators. Its data, glyphs, and desktop states reflect the implementation and fixture. The concept illustrates a clock and controller/gamepad hints while the running UI shows available Qt controls and supported input.

See [responsive evidence](phase-4-2-1-responsive-layout.md) and [validation](phase-4-2-1-validation.md).

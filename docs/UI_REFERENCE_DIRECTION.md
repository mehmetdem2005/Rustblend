# RustBlend 0.5.0 — reference-led workspace

The five user references supplied on 2026-11-11 define the visual direction:
charcoal panels, orange active tools, a dominant 3D viewport, left tool rail,
scene collection and inspector on the right, compact project and status bars.
They are aspirational design references, not screenshots of implemented features.

## This device-review milestone

- Surface-scaled shared layout for rendering and touch hit targets.
- Explicit orbit, move, rotate, scale, add, copy and delete tool rail.
- Project browser in the top bar; undo/redo along the bottom.
- Collapsible scene and transform inspector with actual position, scale and
  quaternion values. Quaternion fields are labelled Q, not Euler angles.
- Scene pagination follows the active object; PREV/NEXT select the first object
  of the neighbouring page, making objects beyond the first screen reachable.
- Reset Transform resets the active object only, is undoable and participates
  in the existing recovery path.
- MESH PREVIEW honestly names the current edge/vertex display. It is not mesh
  editing, sculpting, UV unwrapping, a material editor or animation.

## Still required

Typography with real Unicode/Turkish fonts, density/inset-aware native layout,
viewport framing/scissoring around panels, gesture/device accessibility review,
real picking/gizmos, numeric input, material pipeline and mesh editing remain.
No engine model is bundled. The default scene still contains the existing cube.
The screenshots do not imply million-polygon or photoreal rendering support.

## APK review

0.5.0 / versionCode 6 is an ARM64 device-review build. The workflow runs unit
and Android compilation checks before packaging, then verifies APK signatures,
package metadata and 16 KB alignment. A push containing `[review-apk]` opts into
packaging; ordinary pushes remain check-only. Manual build_apk remains available.
CI uses an ephemeral signing key: this build may not install over an older APK.
Do not uninstall an installation containing projects without preserving them;
external project export and stable release signing are still open work.
Physical-device launch, touch, pause/resume and GPU compatibility require review.

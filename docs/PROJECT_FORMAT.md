# RUSTBLEND project format v1 and automatic recovery

The first persistence milestone stores the current workspace in app-private
`projects/recovery-0.rblend` and `projects/recovery-1.rblend` files. Android obtains
its private directory from `AndroidApp::internal_data_path()`. No storage
permission, network service or new account is required.

A touch gesture ending (including camera navigation), cancellation, suspension,
and a close request queue a save on a single background writer. Startup reads the newest valid generation.
The footer shows NOT SAVED, SAVING, SAVED, SAVE FAILED, RESTORED or RECOVERY BLOCKED.
An interrupted gesture may lose changes since the previous completed save.
The PROJECTS panel now manages private named snapshots; see PROJECT_MANAGER.md. External file picking, importing and sharing are not yet implemented.
Uninstall/clear app data deletes private recovery. The current temporary APK
signing policy still makes upgrades problematic; private recovery is not an
external backup and must not be described as one.

## Binary layout

All integer and floating-point fields are little-endian. Header:

| Offset | Type | Meaning |
|---|---|---|
| 0 | 8 bytes | ASCII RBLEND01 |
| 8 | u32 | Version, currently 1 |
| 12 | u64 | Recovery generation |
| 20 | u64 | Payload bytes |
| 28 | u64 | FNV-1a of bytes 0..28 followed by payload |
| 36 | bytes | Payload |

Payload order: next object ID; u32 object count; for each object its u64 ID,
u32 byte-length UTF-8 name, position (3 f32), rotation xyzw (4 f32), scale
(3 f32), u32 vertex count and positions, u32 edge count and u32 index pairs,
u32 face count and each face's u32 corner count and indices. Then u32 selection
count and u64 selected IDs, u64 active ID (0 = none), camera target (3 f32),
distance/aspect/yaw/pitch (4 f32), tool (u8: orbit=0, move=1, rotate=2, scale=3),
edit flag and outliner flag (each u8, 0/1).

IDs, selection order, geometry and transforms survive reload. Quaternion
normalization on decode can introduce normal floating-point rounding. Camera
aspect is adjusted to the current viewport when GPU resources are recreated.
History is preserved for in-process suspend/resume, but deliberately not written
to disk; a new process starts a fresh undo history.

## Safety and limits

- Maximum total file: 8 MiB; 10,000 objects; 4,096 UTF-8 bytes per name;
  4,096 corners per face. These are provisional safety limits, not performance
  claims. Oversized saves fail without replacing the last committed project.
- Decode validates length, checksum, version, finite numbers, unit rotations,
  camera parameters, mesh indices and scene identity/selection invariants.
- FNV detects accidental corruption, not malicious tampering or authenticity.
- Saves write and sync `recovery.tmp`, rename it onto the alternating slot,
  then sync the directory. The preceding generation remains in the other slot.
- Only committed slots are read; abandoned temp files are ignored.
- A corrupt latest slot falls back to the valid older slot with diagnostics.
- If both committed slots fail, recovery writes are blocked. An unsupported
  version in either slot also blocks writes, avoiding silent downgrade.
- Encoding and disk IO run on a single background writer with at most one
  in-flight and one replaceable pending snapshot. New submissions replace only
  pending work; the current write completes before the latest pending write.
- Completion revisions prevent an older save from marking newer work SAVED.
  A native event wakes the idle UI when a write finishes; there is no polling.
- CPU scene snapshot cloning still happens on the UI thread at gesture boundaries.
  Snapshot/delta optimization and actual device latency measurements remain open.
- Normal application teardown drains pending work and joins the writer. Suspend
  queues a snapshot without waiting for disk IO. Android can kill a process before
  a pending write finishes; only the last completed generation is recoverable.
- Android private-filesystem rename behavior is targeted; this is not yet a
  Storage Access Framework save implementation.

## Verification

Unit tests cover exact identity/geometry/selection round-trips, empty scenes,
all truncated prefixes, header/payload corruption, invalid numeric/mesh/reference
values with valid checksums, oversized values, deterministic malformed bytes,
slot generation selection, interrupted temp writes, corrupt-slot fallback,
failed saves, future versions, and preservation of existing recovery files.

Device tests still required: modify and reopen after process termination;
repeat with an empty scene; suspend/resume; inspect SAVE FAILED behavior under
storage pressure. These require a later meaningful APK milestone.

## Demand-driven rendering

The Android loop waits when idle. Input, resize, visibility restoration and save
status changes request frames. Timeout/outdated/lost surfaces retry at 100 ms
intervals, at most three times until a new user/window event. Occluded windows
and validation failures do not spin. Resuming or interacting allows recovery to
be retried. These are code-level policies; battery, thermal and Mali device
behavior still need device measurement.

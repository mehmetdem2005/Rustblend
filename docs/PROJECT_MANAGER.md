# Private project manager — first usable snapshot workflow

Open **PROJECTS** in the viewport footer. **SAVE** updates the active project after backing up its previous version. **COPY** writes a separate
snapshot, **NEW** opens an empty scene, and tapping a project row loads it. PREV
and NEXT paginate the list; CLOSE returns to the viewport. While an operation is
running, scene input is blocked so an asynchronous load cannot discard edits
made after its safety snapshot was captured.

Names are currently automatic: PROJECT 0001, PROJECT 0002, etc. BACKUP entries
are created before a successful New/Open transition and are also openable. The
catalog API accepts UTF-8 display names, but a rename/name-entry UI is not yet
implemented. This avoids pretending the current bitmap UI is a complete text
input system. Opening or copying a project makes it active for this session. SAVE updates that project; COPY creates another named snapshot. Opening the panel shows UNSAVED when the scene differs from its last named save.

## Data and failure behavior

- Named files live separately in Android private `named-projects`; the previous
  two-slot automatic recovery remains in `projects`.
- Before switching, the requested file is decoded (Open only), then the current
  workspace is committed as a BACKUP. A failed decode or backup aborts the switch.
- GPU replacement errors attempt to restore the prior CPU scene, camera and undo
  history. The independently saved BACKUP remains available as well.
- A file is written and synced as pending.tmp, atomically published with a
  no-clobber hard link, then the temporary link is removed and the directory
  synced. COPY never overwrites existing projects. SAVE backs up the previous valid workspace, writes and syncs update.tmp, then atomically renames it over the active file.
- This relies on Android private-filesystem hard-link support; unsupported
  filesystems fail without replacing the live scene. This is not a SAF exporter.
- IDs, not user names, generate paths. Unicode names and path-like display text
  cannot change the destination directory. Catalog scans reject symlink entries.
- Newest IDs appear first. Unreadable headers are shown as UNREADABLE entries;
  opening still validates the full project. Failed operations appear in the panel.
- Only one explicit project job runs at a time. Jobs are not silently coalesced.
  Listing, loading, encoding and disk IO run on a dedicated background worker.
- Scene snapshots are still cloned on the UI thread. Thumbnail loading, rename,
  delete/archive, custom names, external backup and performance tuning are pending.
- Provisional capacity: 256 snapshots including backups, 120 UTF-8 bytes per name,
  and the existing 8 MiB inner project limit. A full catalog refuses further
  save/switch operations. A safe deletion/archive workflow is a follow-up task.
- These files are not external backups. Uninstalling or clearing app data removes
  them. The current temporary APK signing scheme remains unsuitable for durable
  in-place upgrades until persistent signing is configured.

## Catalog envelope

Each numeric-ID .rblend file contains 8-byte RBCAT001 magic, u32 little-endian
name byte length, UTF-8 name, then the complete checksummed v1 project payload
specified in PROJECT_FORMAT.md. Display-name bytes are metadata outside the
inner checksum. Unsupported or damaged headers cannot be opened. Incomplete
pending.tmp files are never listed as projects.

## Verification

Tests cover immutable copies, restart/open round-trips, Unicode and path-like
names, invalid names, failed writes, corrupt files, abandoned temporary files,
backup-before-switch, backup failure blocking a new scene, nonexistent project
opening, single-flight service completion, panel hit targets, pagination,
interaction blocking and finite overlay geometry.

Device acceptance remains pending: save multiple scenes, load each, verify old
work in BACKUP, New to empty scene, suspend during a project job, process restart,
storage failure and actual private-filesystem hard-link support. No APK is
produced for this source-only milestone.

## Save the active project

SAVE updates the currently opened project ID and keeps its name. Before atomic
replacement, the previous committed workspace is saved as a separately openable
BACKUP. Failed backup or write aborts the replacement. COPY still creates a new
project and makes it active. NEW clears the active project. The active name is
shown in the panel status after operations. Active identity is session-only for
now: after process restart, recovered work is unnamed and SAVE creates a copy.
The panel shows scene changes as UNSAVED; camera-only changes are not included. Persistent active identity, custom name input and
explicit restore-version UI remain pending. Backups count toward the 256 limit.

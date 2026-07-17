# Reusable engineering lessons

## Filesystem mutation

- Before recursive deletion, validate the exact relative path shape and canonical containment; a string prefix check is not enough. Back up the full logical object, reject unsupported symlinks, verify the backup's required anchor file, and only then delete.
- Backup directory names need collision resistance beyond second-resolution timestamps. Use exclusive creation plus a counter/random component and retry on collision.

## Asynchronous editors

- Bind delayed results and confirmations to an immutable target path plus editor revision. A result for another path or revision is stale and must not be applied.
- Classify background work as read-only or mutating. Mutating work must not start over dirty/conflicted state, must lock competing editor actions, and must reload or conflict-block the bound document on completion or channel failure.

## Dry-run and validation boundaries

- A dry run must suppress every side effect, including backups and logs, not only the primary file write.
- Validate generated candidates before persistence. If fallback is needed, reload the last trusted disk version rather than repairing an in-memory candidate that already failed validation.

## Bulk data and consolidated outputs

- Checked bulk scans should return successful items and explicit diagnostics; silently filtering parse failures can turn incomplete input into a false clean result.
- For many inputs targeting one consolidated file, read once, merge with stable idempotent markers, and backup/write at most once when content changed.
- Derive import identity from the source-relative path, not only a filename stem; detect batch collisions and use a deterministic suffix when necessary.

## Bulk synchronization acceptance

- Treat hash equality, identity preservation, and semantic normalization as three independent acceptance dimensions. Per-path hashes prove byte equality, the identity key/path set proves object continuity, and field-level comparison reveals user-visible normalization; passing one does not imply the others.
- Apply the authoritative source validator contract before inventing a competing field invariant. When that contract requires normalization, explicitly disclose the semantic and UI-visible changes instead of hiding them behind validator success or hash equality.

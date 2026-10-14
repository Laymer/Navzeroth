# Local database backup and recovery

The native desktop's **Backup and recovery** screen is available after a database
startup failure as well as through ordinary navigation. Startup checking runs on
a background worker; wait for it to finish before inspecting or restoring files.
The browser demo does not read, back up or replace databases.

## Backup

Choose a new local file path and select **Create consistent backup**. Existing
files are never overwritten. This uses SQLite's supported online backup API,
including committed WAL pages; copying only snapshots.sqlite3 while the app is
running is not a substitute. Keep backups private: they contain imported profiles,
content, parties, session events and feature preferences/history.

**Check database integrity** checks structural integrity, foreign keys and stored
identities/documents. Refusal is an error, not permission to discard the database.

## Explicit restore

Close other application instances. Choose a compatible local SQLite backup and
select **Inspect backup**. A supported schema (1–8) is reconstructed in a fresh
trusted database, migrated to the current schema and validated. The application
never executes SQL supplied by the backup. Unknown/altered schemas, extra objects,
future or inconsistent migration histories, invalid documents/hashes/pointers and
exceeded capacities are refused without replacing current application data.

Startup checking uses the same bounded source validation. The input and its
WAL/SHM sidecars together must fit within 512 MiB. Reconstruction
accepts at most one million rows, at most 32 MiB per copied row and existing
per-feature bounds. These limits are explicit refusals, never truncation. SQLite
can manage SHM locks for a valid WAL input; application records are read-only.

Review the schema and retained record counts. Checking **Replace all local database
state with this inspected backup** enables **Restore inspected backup**. This
replaces all profiles, content and feature history, rather than merging records.
Changing the selected source clears the preview. Restore uses the staged immutable
candidate identity; the original source path is not read again during replacement.

Restore waits for active application operations, stops and joins file watching,
and takes a final consistent backup when the current database is healthy. After
closing its SQLite connection, it retains the original database, WAL and SHM in a
new sibling recovery directory. The screen reports that directory after success.
Watching stays stopped; start a new watch explicitly if wanted.

A failed filesystem replacement rolls back moved original files. An interrupted
restore leaves a transaction marker and retained recovery-directory path visible
on the next startup; the application does not silently create an empty database.
Inspect a compatible retained backup to recover explicitly. Retained original
files and consistent backups are not automatically deleted. A failed marker cleanup
is reported visibly even if the database replacement succeeds; the next startup
continues to require explicit recovery while that marker remains.

## Evidence boundary

Synthetic tests cover populated supported schemas, retained current feature state,
corrupt/malicious/future inputs, committed WAL reads, explicit restore identity,
watcher shutdown, operation leases, injected replacement failure and interruption.
Mocked browser tests cover the user workflow and command argument shapes.
Actual native Windows filesystem replacement, installer/uninstall interaction and
power-loss behavior remain unverified. This feature does not perform game actions,
read live memory or create an addon network connection.

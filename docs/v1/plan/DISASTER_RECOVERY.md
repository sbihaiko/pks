# PKS Disaster Recovery

## Overview

The PKS daemon maintains binary snapshots of indexed vault state under `~/.pks/snapshots/`.
When optionally configured, snapshots are also mirrored to a Git LFS satellite repository
at `~/.pks/lfs-snapshots/` and pushed to `PKS_VECTOR_REMOTE_URL`.

Recovery is always possible. The worst case is a full reindex, which typically completes
in under 60 seconds for a vault of normal size.

---

## Scenario A — Recovery with LFS Satellite

Use when `PKS_VECTOR_REMOTE_URL` was configured and pushes were succeeding.

### Prerequisites

- `git` and `git-lfs` installed and authenticated against the remote
- `PKS_VECTOR_REMOTE_URL` set in the environment

### Steps

```bash
mkdir -p ~/.pks/lfs-snapshots
cd ~/.pks/lfs-snapshots

git init
git lfs install
git remote add origin "$PKS_VECTOR_REMOTE_URL"

git fetch origin snapshots
git checkout -b snapshots origin/snapshots

ls ~/.pks/lfs-snapshots/*.bin
```

After the `.bin` files are present under `~/.pks/lfs-snapshots/`, copy them to the
snapshots directory:

```bash
cp ~/.pks/lfs-snapshots/*.bin ~/.pks/snapshots/
```

Restart the PKS daemon. It will load snapshots from `~/.pks/snapshots/` on startup
without triggering a full reindex.

### Benchmark target

LFS pull completes in under 5 seconds per repository on a typical broadband connection.

---

## Scenario B — Recovery without LFS Satellite

Use when `PKS_VECTOR_REMOTE_URL` was not configured, or when the LFS remote is
unavailable or quota-exceeded.

### Steps

```bash
export PKS_VAULTS_DIR=/path/to/your/vaults

pks reindex --all
```

This clears the in-memory index and walks all registered vaults, rebuilding BM25 and
vector indexes from scratch.

Alternatively, if only a single vault needs recovery:

```bash
pks reindex --vault <vault-name>
```

### Benchmark target

Local reindex completes in under 5 seconds for a vault with up to 500 markdown files.
A full multi-vault reindex completes in under 60 seconds under normal I/O conditions.

---

## Failure Scenarios

### git-lfs not installed

**Symptom**: `LFS repo init failed (non-fatal): Git command failed: git: 'lfs' is not a
command`

**Effect**: PKS daemon continues running. Snapshots are saved locally. Remote sync is
silently skipped.

**Resolution**: Install `git-lfs` via your system package manager, then restart the daemon.

```bash
brew install git-lfs
git lfs install
```

### LFS quota exceeded

**Symptom**: Push step logs `LFS push failed (non-fatal): Git command failed: batch
response: LFS: insufficient quota`

**Effect**: PKS daemon continues running. Local snapshots remain intact. Only remote
backup is affected.

**Resolution**:

1. Increase quota on the LFS host, or
2. Prune old LFS objects on the remote:

```bash
cd ~/.pks/lfs-snapshots
git lfs prune
git push --force origin HEAD:snapshots
```

### Corrupt local snapshot

**Symptom**: `SnapshotError::SchemaMismatch` or `SnapshotError::VersionMismatch` in logs.

**Resolution**: Delete the affected snapshot file and trigger reindex.

```bash
VAULT=my-vault
SAFE="${VAULT//\//_}"
rm ~/.pks/snapshots/"${SAFE}.bin"
pks reindex --vault "$VAULT"
```

---

## Environment Variables Reference

| Variable | Default | Purpose |
|---|---|---|
| `PKS_SNAPSHOTS_DIR` | `~/.pks/snapshots` | Local snapshot storage path |
| `PKS_VECTOR_REMOTE_URL` | (unset) | Git remote URL for LFS satellite; disables LFS sync when unset |
| `PKS_BACKUP_COMPRESS` | `false` | Enable zstd compression on snapshots (stub — logs warning, not yet implemented) |
| `PKS_SNAPSHOT_INTERVAL_COMMITS` | `100` | Commits between automatic snapshot saves |
| `PKS_SNAPSHOT_INTERVAL_SECS` | `300` | Seconds between automatic snapshot saves |

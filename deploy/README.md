# PKS Daemon — Service Installation

PKS runs as an MCP stdio server (`pks --stdio`).
These instructions install it as a persistent OS service that starts on boot and restarts on crash.

---

## Prerequisites

1. Build the release binary:

   ```bash
   cd pks
   cargo build --release
   ```

2. Create the log directory:

   ```bash
   mkdir -p ~/.pks/logs
   ```

3. Create an `.env` file in the working directory (see env vars below).

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PKS_VAULTS_DIR` | `~/VSCodeProjects` | Root directory scanned for vaults |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama API endpoint |
| `OLLAMA_EMBED_MODEL` | `nomic-embed-text` | Embedding model name |
| `PKS_EMBEDDING_PROVIDER` | `ollama` | Embedding backend (`ollama` or `none`) |
| `PKS_LOG_MAX_SIZE` | `10485760` | Max log file size in bytes (10 MB) |

---

## macOS — launchd

### 1. Copy the binary

```bash
sudo cp pks/target/release/pks /usr/local/bin/pks
sudo chmod +x /usr/local/bin/pks
```

### 2. Edit the plist

Open `pks/deploy/com.pks.daemon.plist` and adjust:

- `WorkingDirectory` — must point to the directory containing your `.env` file.
- `PKS_VAULTS_DIR` — root path of your knowledge vaults.
- `StandardOutPath` / `StandardErrorPath` — log destinations (default: `~/.pks/logs/`).

### 3. Install and start the service

```bash
cp pks/deploy/com.pks.daemon.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.pks.daemon.plist
launchctl start com.pks.daemon
```

### 4. Verify

```bash
launchctl list | grep pks
tail -f ~/.pks/logs/pks.stderr.log
```

### 5. Stop / uninstall

```bash
launchctl stop com.pks.daemon
launchctl unload ~/Library/LaunchAgents/com.pks.daemon.plist
```

### Notes

- `KeepAlive=true` — launchd restarts PKS automatically if it crashes.
- `RunAtLoad=true` — PKS starts when the user logs in.
- `ExitTimeout=30` — launchd sends SIGTERM and waits 30 s before SIGKILL.
- `ThrottleInterval=10` — prevents tight crash loops.

---

## Linux — systemd

### System-wide installation (root)

```bash
sudo cp pks/target/release/pks /usr/local/bin/pks
sudo chmod +x /usr/local/bin/pks
sudo mkdir -p /opt/pks
sudo cp pks/deploy/pks.service /etc/systemd/system/pks.service
```

Create `/opt/pks/.env`:

```ini
PKS_VAULTS_DIR=/home/youruser/VSCodeProjects
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_EMBED_MODEL=nomic-embed-text
PKS_EMBEDDING_PROVIDER=ollama
PKS_LOG_MAX_SIZE=10485760
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable pks
sudo systemctl start pks
```

### Per-user installation (no root)

```bash
mkdir -p ~/.config/systemd/user
cp pks/deploy/pks.service ~/.config/systemd/user/pks.service
# Edit WorkingDirectory, EnvironmentFile, and log paths to use $HOME
systemctl --user daemon-reload
systemctl --user enable pks
systemctl --user start pks
```

### Verify

```bash
systemctl status pks
journalctl -u pks -f
# or tail the log file directly:
tail -f ~/.pks/logs/pks.stderr.log
```

### Stop / uninstall

```bash
sudo systemctl stop pks
sudo systemctl disable pks
sudo rm /etc/systemd/system/pks.service
sudo systemctl daemon-reload
```

### Notes

- `Restart=always` — systemd restarts PKS on any exit.
- `RestartSec=10` — waits 10 s between restart attempts.
- `TimeoutStopSec=30` + `KillMode=mixed` — sends SIGTERM, waits 30 s, then SIGKILL.
- `ProtectSystem=strict` + `ReadWritePaths` — basic filesystem sandboxing.

---

## Troubleshooting

| Symptom | Check |
|---------|-------|
| Service not starting | Binary path correct? `which pks` |
| No embeddings | `OLLAMA_BASE_URL` reachable? `curl http://localhost:11434` |
| `.env` not loaded | `WorkingDirectory` must be the directory containing `.env` |
| Logs not written | `~/.pks/logs/` directory exists and is writable? |

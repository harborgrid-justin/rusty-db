# RustyDB systemd Service Templates

This directory contains systemd service unit files for deploying RustyDB on Linux systems.

## Files

- `rustydb@.service` - Template unit for multi-instance deployments
- `rustydb-single.service` - Single instance deployment

## Prerequisites

1. **Create rustydb user:**
   ```bash
   sudo useradd -r -s /bin/false -d /var/lib/rustydb rustydb
   ```

2. **Create directory structure:**
   ```bash
   sudo mkdir -p /opt/rustydb/current/bin
   sudo mkdir -p /var/lib/rustydb/instances
   sudo chown -R rustydb:rustydb /var/lib/rustydb
   ```

3. **Install binary:**
   ```bash
   sudo cp target/release/rusty-db-server /opt/rustydb/current/bin/
   sudo cp target/release/rusty-db-cli /opt/rustydb/current/bin/
   sudo chmod +x /opt/rustydb/current/bin/*
   ```

## Installation

### Multi-Instance Deployment (Recommended)

```bash
# Copy template unit
sudo cp rustydb@.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload

# Create instance directory
sudo -u rustydb mkdir -p /var/lib/rustydb/instances/prod
sudo -u rustydb cp /opt/rustydb/current/conf/rustydb.toml /var/lib/rustydb/instances/prod/conf/

# Enable and start instance
sudo systemctl enable --now rustydb@prod
```

### Single Instance Deployment

```bash
# Copy single instance unit
sudo cp rustydb-single.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload

# Create default directory
sudo -u rustydb mkdir -p /var/lib/rustydb/default/conf
sudo -u rustydb cp /opt/rustydb/current/conf/rustydb.toml /var/lib/rustydb/default/conf/

# Enable and start
sudo systemctl enable --now rustydb-single
```

## Usage

### Multi-Instance Commands

```bash
# Start instance
sudo systemctl start rustydb@prod

# Stop instance
sudo systemctl stop rustydb@prod

# Restart instance
sudo systemctl restart rustydb@prod

# Check status
sudo systemctl status rustydb@prod

# View logs
sudo journalctl -u rustydb@prod -f

# Enable at boot
sudo systemctl enable rustydb@prod

# Disable at boot
sudo systemctl disable rustydb@prod
```

### Multiple Instances

```bash
# Create multiple instances
for instance in prod staging dev; do
    sudo -u rustydb mkdir -p /var/lib/rustydb/instances/$instance/conf
    sudo systemctl enable --now rustydb@$instance
done

# Check all instances
sudo systemctl list-units 'rustydb@*'
```

## Instance Directory Structure

Each instance follows the Instance Layout Spec v1.0:

```
/var/lib/rustydb/instances/<instance>/
├── conf/
│   ├── rustydb.toml          # Main configuration
│   ├── overrides.d/          # Override files (optional)
│   └── secrets/              # Sensitive files (TLS certs, auth)
├── data/
│   ├── meta/                 # Instance metadata
│   │   ├── layout-version
│   │   ├── instance-id
│   │   ├── created-at
│   │   └── data-format-version
│   └── ...                   # Database files
├── logs/                     # Log files
├── run/                      # PID files, sockets
├── cache/                    # Cache (disposable)
├── tmp/                      # Temporary files
├── backup/                   # Backup storage
└── diag/                     # Diagnostics
```

## Security Notes

1. **File Permissions:**
   - Instance directories should be owned by `rustydb:rustydb`
   - Permissions should be `750` for directories, `640` for files
   - Secrets directory should be `700` with `600` for files

2. **Network:**
   - By default, RustyDB binds to `127.0.0.1:54321`
   - For network access, update `listen_host` in config
   - Consider firewall rules for production

3. **systemd Hardening:**
   - Services run with `NoNewPrivileges=true`
   - Only instance directory is writable
   - System directories are protected

## Troubleshooting

### Service won't start

```bash
# Check logs
sudo journalctl -u rustydb@prod -n 100 --no-pager

# Check configuration
sudo -u rustydb /opt/rustydb/current/bin/rusty-db-cli config validate \
    --home /var/lib/rustydb/instances/prod
```

### Permission denied

```bash
# Fix ownership
sudo chown -R rustydb:rustydb /var/lib/rustydb/instances/prod

# Fix SELinux (if applicable)
sudo restorecon -Rv /var/lib/rustydb
```

### Port already in use

```bash
# Check what's using the port
sudo ss -tlnp | grep 54321

# Use different port in config
[server]
listen_port = 54322
```

## Resource Limits

The service files include generous default limits. Adjust as needed:

```ini
# In service file or drop-in
[Service]
LimitNOFILE=2097152
MemoryMax=8G
CPUQuota=400%
```

Create a drop-in override:
```bash
sudo systemctl edit rustydb@prod
```

## Version Management

For zero-downtime upgrades, use symlinks:

```bash
# Install new version
sudo mkdir -p /opt/rustydb/0.3.001/bin
sudo cp target/release/rusty-db-* /opt/rustydb/0.3.001/bin/

# Switch version
sudo ln -sfn /opt/rustydb/0.3.001 /opt/rustydb/current

# Restart instances
sudo systemctl restart 'rustydb@*'
```

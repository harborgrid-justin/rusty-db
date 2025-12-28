# RustyDB v0.6.0 - Installation Guide

**Document Version**: 1.0
**Release**: v0.6.0
**Last Updated**: 2025-12-28
**Classification**: Enterprise Operations
**Target Audience**: System Administrators, DevOps Engineers

---

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Pre-Installation Planning](#pre-installation-planning)
3. [Linux Installation](#linux-installation)
4. [Windows Installation](#windows-installation)
5. [Docker Installation](#docker-installation)
6. [Source Installation](#source-installation)
7. [Post-Installation Verification](#post-installation-verification)
8. [Initial Configuration](#initial-configuration)
9. [Troubleshooting Installation](#troubleshooting-installation)

---

## System Requirements

### Minimum Requirements (Development/Testing)

**Hardware**:
- CPU: 2 cores (x86-64)
- RAM: 4 GB
- Storage: 20 GB available disk space
- Network: 100 Mbps

**Software**:
- Linux kernel: 3.2.0 or later
- Operating System:
  - Ubuntu 20.04 LTS or later
  - RHEL/CentOS 8 or later
  - Debian 11 or later
  - Windows Server 2019 or later (experimental)

### Recommended Requirements (Production)

**Hardware**:
- CPU: 8-16 cores (x86-64 with AVX2/AVX-512)
- RAM: 32-64 GB
- Storage: 500 GB - 2 TB NVMe SSD (RAID 10 recommended)
- Network: 10 Gbps

**Software**:
- Linux kernel: 5.1+ (for io_uring support)
- Operating System:
  - Ubuntu 22.04 LTS (recommended)
  - RHEL 9
  - Rocky Linux 9

### Storage Requirements

**Disk Space Calculation**:
```
Total Required = Data Size + WAL + Backups + Overhead

Example:
- Data files: 100 GB
- WAL logs: 10 GB (10% of data)
- Backups: 200 GB (2x data size)
- Overhead: 20 GB (temp files, cache)
Total: 330 GB
```

**I/O Requirements**:
- SSD minimum for data files
- NVMe recommended for production
- Sustained IOPS: 10,000+ (SSD), 100,000+ (NVMe)
- Read/Write latency: < 10ms (SSD), < 1ms (NVMe)

---

## Pre-Installation Planning

### Network Planning

**Required Ports**:
```
5432   - PostgreSQL protocol (database connections)
8080   - REST API (HTTP)
9100   - Prometheus metrics
54321  - Alternative database port (configurable)
```

**Firewall Configuration**:
- Allow inbound connections on database port from application servers
- Allow inbound connections on API port from management network
- Allow outbound connections for replication (if clustering)

### User and Group Setup

All installations require a dedicated system user:

**User**: `rustydb`
**Group**: `rustydb`
**Home Directory**: `/var/lib/rustydb`
**Shell**: `/bin/false` (no login)

---

## Linux Installation

### Method 1: Binary Installation (Recommended)

#### Ubuntu/Debian

**Step 1: Create System User**
```bash
sudo useradd -r -s /bin/false -d /var/lib/rustydb rustydb
```

**Step 2: Create Directory Structure**
```bash
sudo mkdir -p /opt/rustydb/current/bin
sudo mkdir -p /var/lib/rustydb/instances/default
sudo mkdir -p /var/log/rustydb
```

**Step 3: Download and Install Binaries**
```bash
# Download release package
cd /tmp
wget https://github.com/rustydb/rusty-db/releases/download/v0.6.0/rustydb-0.6.0-linux-x86_64.tar.gz

# Extract binaries
tar -xzf rustydb-0.6.0-linux-x86_64.tar.gz
cd rustydb-0.6.0-linux-x86_64

# Install binaries
sudo cp rusty-db-server /opt/rustydb/current/bin/
sudo cp rusty-db-cli /opt/rustydb/current/bin/
sudo chmod +x /opt/rustydb/current/bin/*

# Create symlinks for easy access
sudo ln -sf /opt/rustydb/current/bin/rusty-db-server /usr/local/bin/
sudo ln -sf /opt/rustydb/current/bin/rusty-db-cli /usr/local/bin/
```

**Step 4: Install Configuration**
```bash
# Copy default configuration
sudo mkdir -p /var/lib/rustydb/instances/default/conf
sudo cp conf/rustydb.toml /var/lib/rustydb/instances/default/conf/

# Set ownership
sudo chown -R rustydb:rustydb /var/lib/rustydb
sudo chown -R rustydb:rustydb /var/log/rustydb
```

**Step 5: Install systemd Service**
```bash
# Copy service file
sudo cp deploy/systemd/rustydb-single.service /etc/systemd/system/rustydb.service

# Reload systemd
sudo systemctl daemon-reload

# Enable service
sudo systemctl enable rustydb
```

**Step 6: Start Service**
```bash
sudo systemctl start rustydb
```

#### RHEL/CentOS/Rocky Linux

**Step 1-4**: Same as Ubuntu/Debian above

**Step 5: Configure SELinux (if enabled)**
```bash
# Set SELinux contexts
sudo semanage port -a -t postgresql_port_t -p tcp 5432
sudo semanage port -a -t http_port_t -p tcp 8080

# Set file contexts
sudo semanage fcontext -a -t bin_t "/opt/rustydb/current/bin(/.*)?"
sudo semanage fcontext -a -t postgresql_db_t "/var/lib/rustydb(/.*)?"
sudo semanage fcontext -a -t postgresql_log_t "/var/log/rustydb(/.*)?"

# Apply contexts
sudo restorecon -Rv /opt/rustydb
sudo restorecon -Rv /var/lib/rustydb
sudo restorecon -Rv /var/log/rustydb
```

**Step 6**: Install and start service (same as Ubuntu)

### Method 2: Package Manager Installation (Future)

**Note**: Package repository integration is planned for future releases.

**Ubuntu/Debian (Planned)**:
```bash
# Add repository
curl -fsSL https://packages.rustydb.io/gpg | sudo gpg --dearmor -o /usr/share/keyrings/rustydb-archive-keyring.gpg

echo "deb [signed-by=/usr/share/keyrings/rustydb-archive-keyring.gpg] https://packages.rustydb.io/apt stable main" | \
  sudo tee /etc/apt/sources.list.d/rustydb.list

# Update and install
sudo apt-get update
sudo apt-get install rustydb
```

**RHEL/CentOS (Planned)**:
```bash
# Add repository
sudo cat > /etc/yum.repos.d/rustydb.repo << 'EOF'
[rustydb]
name=RustyDB Repository
baseurl=https://packages.rustydb.io/rpm/stable/$basearch
enabled=1
gpgcheck=1
gpgkey=https://packages.rustydb.io/gpg
EOF

# Install
sudo yum install rustydb
```

### Multi-Instance Installation

For running multiple instances on the same server:

```bash
# Create instances
for instance in prod staging dev; do
    sudo -u rustydb mkdir -p /var/lib/rustydb/instances/$instance/conf
    sudo -u rustydb mkdir -p /var/lib/rustydb/instances/$instance/data
    sudo -u rustydb mkdir -p /var/lib/rustydb/instances/$instance/logs

    # Copy and customize configuration
    sudo -u rustydb cp /opt/rustydb/current/conf/rustydb.toml \
        /var/lib/rustydb/instances/$instance/conf/
done

# Install template service unit
sudo cp deploy/systemd/rustydb@.service /etc/systemd/system/
sudo systemctl daemon-reload

# Start instances
sudo systemctl enable --now rustydb@prod
sudo systemctl enable --now rustydb@staging
sudo systemctl enable --now rustydb@dev
```

**Configure Different Ports**:
Edit each instance's configuration file:
- prod: `/var/lib/rustydb/instances/prod/conf/rustydb.toml` → `listen_port = 54321`
- staging: `/var/lib/rustydb/instances/staging/conf/rustydb.toml` → `listen_port = 54322`
- dev: `/var/lib/rustydb/instances/dev/conf/rustydb.toml` → `listen_port = 54323`

---

## Windows Installation

### Prerequisites

**Required Software**:
- Windows Server 2019 or later
- Administrator privileges
- Visual C++ Redistributable 2019 or later

**Create Service Account (Optional but Recommended)**:
```batch
net user rustydb_svc SecurePassword123! /add
net localgroup "Users" rustydb_svc /add
```

### Installation Steps

**Step 1: Create Directory Structure**
```batch
mkdir "C:\Program Files\RustyDB\current\bin"
mkdir "C:\ProgramData\RustyDB\instances\default\conf"
mkdir "C:\ProgramData\RustyDB\instances\default\data"
mkdir "C:\ProgramData\RustyDB\instances\default\logs"
```

**Step 2: Download and Extract Binaries**
```batch
REM Download from GitHub releases
REM Extract to temporary location

REM Copy binaries
copy rusty-db-server.exe "C:\Program Files\RustyDB\current\bin\"
copy rusty-db-cli.exe "C:\Program Files\RustyDB\current\bin\"
```

**Step 3: Install Configuration**
```batch
copy conf\rustydb.toml "C:\ProgramData\RustyDB\instances\default\conf\"
```

**Step 4: Set Permissions**
```batch
REM Grant service account permissions
icacls "C:\ProgramData\RustyDB" /grant rustydb_svc:(OI)(CI)F /T
icacls "C:\Program Files\RustyDB" /grant rustydb_svc:(OI)(CI)RX /T
```

**Step 5: Install Windows Service**
```batch
REM Navigate to deployment scripts
cd deploy\windows

REM Run installation script
install-service.bat default
```

**Alternative Manual Service Installation**:
```batch
sc create RustyDB_default ^
    binPath= "C:\Program Files\RustyDB\current\bin\rusty-db-server.exe --home C:\ProgramData\RustyDB\instances\default" ^
    DisplayName= "RustyDB Database Server (default)" ^
    start= auto ^
    obj= ".\rustydb_svc" ^
    password= "SecurePassword123!"
```

**Step 6: Configure Firewall**
```batch
REM Add firewall rule
netsh advfirewall firewall add rule ^
    name="RustyDB Server" ^
    dir=in ^
    action=allow ^
    protocol=TCP ^
    localport=5432,8080
```

**Step 7: Start Service**
```batch
sc start RustyDB_default

REM Or using net command
net start RustyDB_default

REM Or using PowerShell
Start-Service -Name RustyDB_default
```

### Multiple Instances on Windows

```batch
REM Install multiple instances
install-service.bat prod
install-service.bat staging
install-service.bat dev

REM Configure different ports in each rustydb.toml
REM prod:    C:\ProgramData\RustyDB\instances\prod\conf\rustydb.toml
REM staging: C:\ProgramData\RustyDB\instances\staging\conf\rustydb.toml
REM dev:     C:\ProgramData\RustyDB\instances\dev\conf\rustydb.toml

REM Start all instances
sc start RustyDB_prod
sc start RustyDB_staging
sc start RustyDB_dev
```

---

## Docker Installation

### Single Container

```bash
# Pull official image
docker pull rustydb/rusty-db:0.6.0

# Run container
docker run -d \
  --name rustydb \
  -p 5432:5432 \
  -p 8080:8080 \
  -v rustydb-data:/var/lib/rustydb \
  -v rustydb-logs:/var/log/rustydb \
  -e RUSTYDB_MODE=prod \
  rustydb/rusty-db:0.6.0

# Check logs
docker logs -f rustydb

# Access CLI
docker exec -it rustydb rusty-db-cli
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  rustydb:
    image: rustydb/rusty-db:0.6.0
    container_name: rustydb-primary
    environment:
      - RUSTYDB_MODE=prod
      - RUSTYDB_INSTANCE=primary
    ports:
      - "5432:5432"
      - "8080:8080"
      - "9100:9100"
    volumes:
      - rustydb-data:/var/lib/rustydb
      - rustydb-logs:/var/log/rustydb
      - ./rustydb.toml:/var/lib/rustydb/instances/default/conf/rustydb.toml:ro
    networks:
      - rustydb-network
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "rusty-db-cli", "--command", "SELECT 1"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  rustydb-data:
  rustydb-logs:

networks:
  rustydb-network:
    driver: bridge
```

**Start with Docker Compose**:
```bash
docker-compose up -d
docker-compose logs -f rustydb
docker-compose ps
```

### Kubernetes Deployment

See separate Kubernetes deployment guide in `DEPLOYMENT_GUIDE.md`.

---

## Source Installation

For advanced users who need to build from source:

### Prerequisites

**Build Dependencies**:
- Rust 1.70 or later
- GCC or Clang
- OpenSSL development libraries
- pkg-config

**Ubuntu/Debian**:
```bash
sudo apt-get update
sudo apt-get install -y build-essential curl libssl-dev pkg-config
```

**RHEL/CentOS**:
```bash
sudo yum groupinstall "Development Tools"
sudo yum install -y openssl-devel pkg-config
```

### Build Steps

**Step 1: Install Rust**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Step 2: Clone Repository**
```bash
git clone https://github.com/rustydb/rusty-db.git
cd rusty-db
git checkout v0.6.0
```

**Step 3: Build Release Binaries**
```bash
# Build with optimizations
cargo build --release

# Binaries will be in target/release/
ls -lh target/release/rusty-db-*
```

**Step 4: Run Tests (Optional)**
```bash
cargo test --release
```

**Step 5: Install Binaries**
```bash
sudo cp target/release/rusty-db-server /opt/rustydb/current/bin/
sudo cp target/release/rusty-db-cli /opt/rustydb/current/bin/
sudo chmod +x /opt/rustydb/current/bin/*
```

**Step 6: Complete Installation**

Follow the same steps as binary installation starting from "Create Directory Structure".

---

## Post-Installation Verification

### Verify Installation

**Check Binary Version**:
```bash
rusty-db-server --version
# Expected: rusty-db-server 0.6.0

rusty-db-cli --version
# Expected: rusty-db-cli 0.6.0
```

**Check Service Status**:
```bash
# Linux
sudo systemctl status rustydb

# Windows
sc query RustyDB_default
```

**Check Listening Ports**:
```bash
# Linux
sudo netstat -tlnp | grep rusty-db
# or
sudo ss -tlnp | grep rusty-db

# Expected output:
# tcp  0  0 127.0.0.1:5432   0.0.0.0:*   LISTEN  <pid>/rusty-db-server
# tcp  0  0 127.0.0.1:8080   0.0.0.0:*   LISTEN  <pid>/rusty-db-server
```

### Test Database Connection

**Using CLI**:
```bash
rusty-db-cli --command "SELECT version();"

# Expected output:
# version
# ---------
# RustyDB 0.6.0
```

**Using REST API**:
```bash
curl http://localhost:8080/api/v1/health

# Expected response:
# {
#   "status": "healthy",
#   "version": "0.6.0",
#   "uptime_seconds": <number>
# }
```

**Using PostgreSQL Client** (if compatible):
```bash
psql -h localhost -p 5432 -U admin -c "SELECT 1;"
```

### Verify File Permissions

**Linux**:
```bash
ls -la /var/lib/rustydb/instances/default/
# All files should be owned by rustydb:rustydb

ls -la /opt/rustydb/current/bin/
# Binaries should be executable (755 or similar)
```

**Windows**:
```batch
icacls "C:\ProgramData\RustyDB\instances\default"
# rustydb_svc should have full control
```

### Verify Logs

**Linux**:
```bash
# systemd logs
sudo journalctl -u rustydb -n 50 --no-pager

# Application logs
sudo tail -50 /var/lib/rustydb/instances/default/logs/rustydb.log
```

**Windows**:
```batch
REM Event Viewer
eventvwr.msc

REM Application logs
type "C:\ProgramData\RustyDB\instances\default\logs\rustydb.log"
```

---

## Initial Configuration

### Create Admin User

```bash
rusty-db-cli --command "
CREATE USER admin WITH PASSWORD 'SecureAdminPassword123!' SUPERUSER;
"
```

### Create Application Database

```bash
rusty-db-cli --command "
CREATE DATABASE myapp;
"
```

### Create Application User

```bash
rusty-db-cli --command "
CREATE USER appuser WITH PASSWORD 'SecureAppPassword123!';
GRANT CONNECT ON DATABASE myapp TO appuser;
"
```

### Configure Firewall

**Ubuntu/Debian (ufw)**:
```bash
sudo ufw allow from 10.0.1.0/24 to any port 5432 proto tcp
sudo ufw allow from 10.0.1.0/24 to any port 8080 proto tcp
sudo ufw reload
```

**RHEL/CentOS (firewalld)**:
```bash
sudo firewall-cmd --permanent --add-rich-rule='
  rule family="ipv4"
  source address="10.0.1.0/24"
  port port="5432" protocol="tcp"
  accept'

sudo firewall-cmd --permanent --add-rich-rule='
  rule family="ipv4"
  source address="10.0.1.0/24"
  port port="8080" protocol="tcp"
  accept'

sudo firewall-cmd --reload
```

### Enable TLS (Optional but Recommended for Production)

**Generate Self-Signed Certificates**:
```bash
cd /var/lib/rustydb/instances/default/conf
sudo mkdir -p secrets/tls

# Generate CA
sudo openssl genrsa -out secrets/tls/ca.key 4096
sudo openssl req -new -x509 -days 3650 -key secrets/tls/ca.key -out secrets/tls/ca.crt

# Generate server certificate
sudo openssl genrsa -out secrets/tls/server.key 4096
sudo openssl req -new -key secrets/tls/server.key -out secrets/tls/server.csr
sudo openssl x509 -req -in secrets/tls/server.csr -CA secrets/tls/ca.crt \
    -CAkey secrets/tls/ca.key -CAcreateserial -out secrets/tls/server.crt -days 365

# Set permissions
sudo chown -R rustydb:rustydb secrets/
sudo chmod 600 secrets/tls/*.key
sudo chmod 644 secrets/tls/*.crt
```

**Update Configuration**:
```bash
sudo nano /var/lib/rustydb/instances/default/conf/rustydb.toml

# Set:
# [tls]
# enabled = true
# cert_path = "secrets/tls/server.crt"
# key_path = "secrets/tls/server.key"

# Restart service
sudo systemctl restart rustydb
```

---

## Troubleshooting Installation

### Issue: Service Fails to Start

**Symptoms**:
```bash
sudo systemctl status rustydb
# Active: failed
```

**Diagnosis**:
```bash
# Check system logs
sudo journalctl -u rustydb -n 100 --no-pager

# Check application logs
sudo tail -100 /var/lib/rustydb/instances/default/logs/rustydb.log

# Check for permission issues
ls -la /var/lib/rustydb/instances/default/
```

**Common Causes**:
1. **Permission denied**: Fix with `sudo chown -R rustydb:rustydb /var/lib/rustydb`
2. **Port already in use**: Check with `sudo lsof -i :5432`
3. **Configuration error**: Validate configuration file
4. **Missing dependencies**: Install required libraries

### Issue: Port Already in Use

**Symptoms**:
```
Error: Address already in use (os error 98)
```

**Diagnosis**:
```bash
# Find what's using the port
sudo lsof -i :5432
sudo ss -tlnp | grep 5432
```

**Solutions**:
1. Stop conflicting service (e.g., PostgreSQL)
2. Change RustyDB port in configuration
3. Configure both services to use different ports

### Issue: Connection Refused

**Symptoms**:
```bash
rusty-db-cli --command "SELECT 1;"
# Error: Connection refused
```

**Diagnosis**:
```bash
# Check if service is running
sudo systemctl status rustydb

# Check if port is listening
sudo netstat -tlnp | grep 5432

# Check firewall
sudo ufw status  # Ubuntu
sudo firewall-cmd --list-all  # RHEL
```

**Solutions**:
1. Start the service: `sudo systemctl start rustydb`
2. Check listen address in config (should be 127.0.0.1 or 0.0.0.0)
3. Add firewall rule to allow connections

### Issue: SELinux Blocking Execution (RHEL/CentOS)

**Symptoms**:
```
Permission denied (SELinux)
```

**Diagnosis**:
```bash
# Check SELinux status
getenforce

# Check audit log
sudo ausearch -m avc -ts recent
```

**Solutions**:
```bash
# Set correct contexts
sudo semanage fcontext -a -t bin_t "/opt/rustydb/current/bin(/.*)?"
sudo restorecon -Rv /opt/rustydb

# Or temporarily disable SELinux (not recommended for production)
sudo setenforce 0
```

### Issue: Binary Not Found

**Symptoms**:
```bash
rusty-db-server: command not found
```

**Solutions**:
```bash
# Check if binary exists
ls -la /opt/rustydb/current/bin/rusty-db-server

# Verify symlink
ls -la /usr/local/bin/rusty-db-server

# Recreate symlink
sudo ln -sf /opt/rustydb/current/bin/rusty-db-server /usr/local/bin/

# Or add to PATH
export PATH="/opt/rustydb/current/bin:$PATH"
```

### Issue: Windows Service Won't Start

**Symptoms**:
```
Error 1053: The service did not respond to the start or control request in a timely fashion
```

**Diagnosis**:
```batch
REM Check Event Viewer
eventvwr.msc

REM Check if binary is executable
"C:\Program Files\RustyDB\current\bin\rusty-db-server.exe" --version

REM Check service configuration
sc qc RustyDB_default
```

**Solutions**:
1. Verify service account has "Log on as a service" right
2. Check file permissions
3. Verify binary path in service configuration
4. Run binary manually to see error messages

---

## Next Steps

After successful installation:

1. **Review Configuration**: See [CONFIGURATION.md](./CONFIGURATION.md)
2. **Set Up Monitoring**: See [MONITORING.md](./MONITORING.md)
3. **Configure Backups**: See [BACKUP_RECOVERY.md](./BACKUP_RECOVERY.md)
4. **Plan Maintenance**: See [MAINTENANCE.md](./MAINTENANCE.md)
5. **Security Hardening**: Review security best practices

---

**Document Maintained By**: Enterprise Documentation Agent 4
**RustyDB Version**: 0.6.0
**Installation Support**: See TROUBLESHOOTING.md for additional help

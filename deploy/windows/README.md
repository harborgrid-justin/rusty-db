# RustyDB Windows Service Deployment

This directory contains scripts for deploying RustyDB as a Windows service.

## Files

- `install-service.bat` - Install RustyDB as a Windows service
- `uninstall-service.bat` - Remove RustyDB service
- `start-service.bat` - Start the service
- `stop-service.bat` - Stop the service

## Prerequisites

1. **Install RustyDB binaries:**
   ```
   C:\Program Files\RustyDB\current\bin\rusty-db-server.exe
   C:\Program Files\RustyDB\current\bin\rusty-db-cli.exe
   ```

2. **Copy default configuration:**
   ```
   C:\Program Files\RustyDB\current\conf\rustydb.toml
   ```

3. **Administrator privileges** are required for service operations.

## Installation

### Quick Start (Default Instance)

1. Open Command Prompt as Administrator
2. Navigate to this directory
3. Run:
   ```batch
   install-service.bat
   ```

### Named Instance

```batch
install-service.bat prod
install-service.bat staging
install-service.bat dev
```

## Directory Structure

Each instance creates the following structure:

```
C:\ProgramData\RustyDB\instances\<instance>\
├── conf\
│   ├── rustydb.toml          # Main configuration
│   ├── overrides.d\          # Override files
│   └── secrets\              # TLS certs, auth files
├── data\
│   └── meta\                 # Instance metadata
├── logs\                     # Log files
├── run\                      # Named pipes, etc.
├── cache\                    # Cache (disposable)
├── tmp\                      # Temporary files
├── backup\                   # Backups
└── diag\                     # Diagnostics
```

## Service Management

### Using sc.exe

```batch
REM Start service
sc start RustyDB_prod

REM Stop service
sc stop RustyDB_prod

REM Query status
sc query RustyDB_prod

REM Restart service
sc stop RustyDB_prod && timeout /t 2 && sc start RustyDB_prod
```

### Using net commands

```batch
net start RustyDB_prod
net stop RustyDB_prod
```

### Using PowerShell

```powershell
# Start
Start-Service -Name RustyDB_prod

# Stop
Stop-Service -Name RustyDB_prod

# Restart
Restart-Service -Name RustyDB_prod

# Status
Get-Service -Name RustyDB_prod
```

## Configuration

Edit the configuration file for your instance:
```
C:\ProgramData\RustyDB\instances\<instance>\conf\rustydb.toml
```

### Key Settings

```toml
[server]
listen_host = "127.0.0.1"  # Change to "0.0.0.0" for network access
listen_port = 54321

[security]
mode = "prod"  # Use "prod" for production

[tls]
enabled = true
cert_path = "secrets/tls/server.crt"
key_path = "secrets/tls/server.key"
```

## Service Account Recommendations

For production deployments:

1. **Create a dedicated service account:**
   ```batch
   net user rustydb_svc <password> /add
   ```

2. **Grant necessary permissions:**
   - Read access to `C:\Program Files\RustyDB`
   - Full control of `C:\ProgramData\RustyDB\instances\<instance>`

3. **Configure service to use the account:**
   ```batch
   sc config RustyDB_prod obj= ".\rustydb_svc" password= "<password>"
   ```

4. **Grant "Log on as a service" right:**
   - Open Local Security Policy (secpol.msc)
   - Navigate to Local Policies > User Rights Assignment
   - Add user to "Log on as a service"

## Firewall Configuration

If accepting network connections:

```batch
REM Add firewall rule
netsh advfirewall firewall add rule ^
    name="RustyDB Server" ^
    dir=in ^
    action=allow ^
    protocol=TCP ^
    localport=54321

REM Remove rule
netsh advfirewall firewall delete rule name="RustyDB Server"
```

Or using PowerShell:
```powershell
New-NetFirewallRule -DisplayName "RustyDB Server" `
    -Direction Inbound -Protocol TCP -LocalPort 54321 -Action Allow
```

## Viewing Logs

### Application Logs
```
C:\ProgramData\RustyDB\instances\<instance>\logs\rustydb.log
```

### Windows Event Log

View in Event Viewer or PowerShell:
```powershell
Get-EventLog -LogName Application -Source RustyDB_prod -Newest 50
```

## Troubleshooting

### Service fails to start

1. Check Event Viewer for errors:
   - Windows Logs > Application
   - Windows Logs > System

2. Verify binary exists:
   ```batch
   dir "C:\Program Files\RustyDB\current\bin\rusty-db-server.exe"
   ```

3. Verify configuration:
   ```batch
   "C:\Program Files\RustyDB\current\bin\rusty-db-cli.exe" config validate ^
       --home "C:\ProgramData\RustyDB\instances\prod"
   ```

4. Check permissions on data directory

### Port already in use

```batch
netstat -ano | findstr :54321
```

Change port in configuration:
```toml
[server]
listen_port = 54322
```

### Service account issues

Reset to LocalSystem for testing:
```batch
sc config RustyDB_prod obj= LocalSystem
```

## Multiple Instances

Run multiple instances on different ports:

```batch
REM Install instances
install-service.bat prod
install-service.bat staging
install-service.bat dev

REM Configure different ports in each instance's rustydb.toml:
REM prod:    54321
REM staging: 54322
REM dev:     54323

REM Start all
sc start RustyDB_prod
sc start RustyDB_staging
sc start RustyDB_dev
```

## Version Upgrades

1. Stop all services:
   ```batch
   sc stop RustyDB_prod
   ```

2. Install new version:
   ```batch
   mkdir "C:\Program Files\RustyDB\0.3.001\bin"
   copy <new_binaries> "C:\Program Files\RustyDB\0.3.001\bin\"
   ```

3. Update symlink (or use junction):
   ```batch
   rmdir "C:\Program Files\RustyDB\current"
   mklink /J "C:\Program Files\RustyDB\current" "C:\Program Files\RustyDB\0.3.001"
   ```

4. Start services:
   ```batch
   sc start RustyDB_prod
   ```

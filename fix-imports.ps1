# PowerShell script to fix missing imports in Rust files
# This script adds missing Mutex, sleep, and interval imports

$ErrorActionPreference = "Stop"
$fixedCount = 0

function Test-NeedsMutexImport {
    param([string]$content)

    $usesMutex = $content -match 'Mutex::(new|lock|try_lock)' -or $content -match 'Mutex<'
    $hasMutexImport = $content -match 'use\s+parking_lot::.*Mutex' -or
                      $content -match 'use\s+std::sync::.*Mutex'

    return $usesMutex -and -not $hasMutexImport
}

function Test-NeedsSleepImport {
    param([string]$content)

    # Check for unqualified sleep( calls (not tokio::time::sleep or std::thread::sleep)
    $lines = $content -split "`n"
    foreach ($line in $lines) {
        if ($line -match '^\s*sleep\(' -or $line -match '\s+sleep\(') {
            if ($line -notmatch 'tokio::time::sleep' -and $line -notmatch 'std::thread::sleep' -and $line -notmatch '//.*sleep') {
                # Check if import exists
                if ($content -notmatch 'use\s+tokio::time::.*sleep') {
                    return $true
                }
            }
        }
    }
    return $false
}

function Test-NeedsIntervalImport {
    param([string]$content)

    # Check for unqualified interval( calls
    $lines = $content -split "`n"
    foreach ($line in $lines) {
        if ($line -match '^\s*interval\(' -or $line -match '\s+interval\(' -or $line -match '=\s*interval\(') {
            if ($line -notmatch 'tokio::time::interval' -and $line -notmatch '//.*interval') {
                # Check if import exists
                if ($content -notmatch 'use\s+tokio::time::.*interval') {
                    return $true
                }
            }
        }
    }
    return $false
}

function Test-HasParkingLot {
    param([string]$content)
    return $content -match 'use\s+parking_lot::'
}

function Add-MutexImport {
    param([string]$content, [bool]$useParkingLot)

    $lines = $content -split "`n"
    $modified = $false

    if ($useParkingLot) {
        # Try to add to existing parking_lot::RwLock line
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match '^use parking_lot::RwLock;') {
                $lines[$i] = 'use parking_lot::{RwLock, Mutex};'
                Write-Host "  Added Mutex to parking_lot import"
                $modified = $true
                break
            }
            elseif ($lines[$i] -match '^use parking_lot::\{' -and $lines[$i] -notmatch 'Mutex') {
                $lines[$i] = $lines[$i] -replace '\}', ', Mutex}'
                Write-Host "  Added Mutex to existing parking_lot multi-import"
                $modified = $true
                break
            }
        }

        if (-not $modified) {
            # Find last use statement and add new import
            for ($i = $lines.Count - 1; $i -ge 0; $i--) {
                if ($lines[$i] -match '^use\s+') {
                    $lines = $lines[0..$i] + "use parking_lot::Mutex;" + $lines[($i+1)..($lines.Count-1)]
                    Write-Host "  Added new parking_lot::Mutex import"
                    $modified = $true
                    break
                }
            }
        }
    }
    else {
        # Use std::sync::Mutex
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match '^use std::sync::\{.*Arc' -and $lines[$i] -notmatch 'Mutex') {
                $lines[$i] = $lines[$i] -replace '\}', ', Mutex}'
                Write-Host "  Added Mutex to std::sync import"
                $modified = $true
                break
            }
        }

        if (-not $modified) {
            for ($i = $lines.Count - 1; $i -ge 0; $i--) {
                if ($lines[$i] -match '^use\s+') {
                    $lines = $lines[0..$i] + "use std::sync::Mutex;" + $lines[($i+1)..($lines.Count-1)]
                    Write-Host "  Added new std::sync::Mutex import"
                    $modified = $true
                    break
                }
            }
        }
    }

    return ($lines -join "`n"), $modified
}

function Add-SleepImport {
    param([string]$content)

    $lines = $content -split "`n"
    $modified = $false

    # Try to add to existing tokio::time import
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^use tokio::time::\{' -and $lines[$i] -notmatch 'sleep') {
            $lines[$i] = $lines[$i] -replace '\}', ', sleep}'
            Write-Host "  Added sleep to existing tokio::time import"
            $modified = $true
            break
        }
    }

    if (-not $modified) {
        # Add new import after tokio imports
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match '^use tokio::') {
                $lines = $lines[0..$i] + "use tokio::time::sleep;" + $lines[($i+1)..($lines.Count-1)]
                Write-Host "  Added new tokio::time::sleep import"
                $modified = $true
                break
            }
        }
    }

    if (-not $modified) {
        # Add after last use statement
        for ($i = $lines.Count - 1; $i -ge 0; $i--) {
            if ($lines[$i] -match '^use\s+') {
                $lines = $lines[0..$i] + "use tokio::time::sleep;" + $lines[($i+1)..($lines.Count-1)]
                Write-Host "  Added tokio::time::sleep import"
                $modified = $true
                break
            }
        }
    }

    return ($lines -join "`n"), $modified
}

function Add-IntervalImport {
    param([string]$content)

    $lines = $content -split "`n"
    $modified = $false

    # Try to add to existing tokio::time import
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^use tokio::time::\{' -and $lines[$i] -notmatch 'interval') {
            $lines[$i] = $lines[$i] -replace '\}', ', interval}'
            Write-Host "  Added interval to existing tokio::time import"
            $modified = $true
            break
        }
        elseif ($lines[$i] -match '^use tokio::time::sleep;') {
            $lines[$i] = 'use tokio::time::{sleep, interval};'
            Write-Host "  Changed sleep import to include interval"
            $modified = $true
            break
        }
    }

    if (-not $modified) {
        # Add new import after tokio imports
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match '^use tokio::') {
                $lines = $lines[0..$i] + "use tokio::time::interval;" + $lines[($i+1)..($lines.Count-1)]
                Write-Host "  Added new tokio::time::interval import"
                $modified = $true
                break
            }
        }
    }

    if (-not $modified) {
        # Add after last use statement
        for ($i = $lines.Count - 1; $i -ge 0; $i--) {
            if ($lines[$i] -match '^use\s+') {
                $lines = $lines[0..$i] + "use tokio::time::interval;" + $lines[($i+1)..($lines.Count-1)]
                Write-Host "  Added tokio::time::interval import"
                $modified = $true
                break
            }
        }
    }

    return ($lines -join "`n"), $modified
}

# Process all Rust files
$rustFiles = Get-ChildItem -Path "src" -Recurse -Filter "*.rs"

foreach ($file in $rustFiles) {
    try {
        $content = Get-Content $file.FullName -Raw -Encoding UTF8
        $originalContent = $content
        $fileModified = $false
        $changes = @()

        # Check and fix Mutex import
        if (Test-NeedsMutexImport $content) {
            $useParkingLot = Test-HasParkingLot $content
            $content, $modified = Add-MutexImport $content $useParkingLot
            if ($modified) {
                $fileModified = $true
                $mutexType = if ($useParkingLot) { "parking_lot" } else { "std::sync" }
                $changes += "Mutex ($mutexType)"
            }
        }

        # Check and fix sleep import
        if (Test-NeedsSleepImport $content) {
            $content, $modified = Add-SleepImport $content
            if ($modified) {
                $fileModified = $true
                $changes += "sleep"
            }
        }

        # Check and fix interval import
        if (Test-NeedsIntervalImport $content) {
            $content, $modified = Add-IntervalImport $content
            if ($modified) {
                $fileModified = $true
                $changes += "interval"
            }
        }

        # Write back if modified
        if ($fileModified) {
            # Ensure content ends with newline
            if (-not $content.EndsWith("`n")) {
                $content += "`n"
            }
            Set-Content -Path $file.FullName -Value $content -Encoding UTF8 -NoNewline
            Write-Host "✓ $($file.FullName): Added $($changes -join ', ')" -ForegroundColor Green
            $fixedCount++
        }
    }
    catch {
        Write-Host "✗ Error processing $($file.FullName): $_" -ForegroundColor Red
    }
}

Write-Host "`nFixed $fixedCount files" -ForegroundColor Cyan
Write-Host "`nRunning cargo check to verify..." -ForegroundColor Cyan
cargo check 2>&1 | Select-String "error" | Select-Object -First 20

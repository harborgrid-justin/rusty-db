# Verification script for flashback and pool module fixes

Write-Host "=" * 80 -ForegroundColor Cyan
Write-Host "VERIFYING FLASHBACK AND POOL MODULE FIXES" -ForegroundColor Cyan
Write-Host "=" * 80 -ForegroundColor Cyan
Write-Host ""

# Check that files were modified
Write-Host "Checking modified files..." -ForegroundColor Yellow
$files = @(
    "src\flashback\time_travel.rs",
    "src\flashback\versions.rs",
    "src\flashback\table_restore.rs",
    "src\flashback\database.rs",
    "src\flashback\transaction.rs",
    "src\pool\session_manager.rs"
)

foreach ($file in $files) {
    $fullPath = Join-Path $PWD $file
    if (Test-Path $fullPath) {
        $content = Get-Content $fullPath -Raw
        if ($content -match 'use crate::error::\{DbError, Result\}') {
            Write-Host "✓ $file - Import pattern correct" -ForegroundColor Green
        } else {
            Write-Host "✗ $file - Import pattern NOT fixed" -ForegroundColor Red
        }
    } else {
        Write-Host "✗ $file - File not found" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "Running cargo check for flashback and pool modules..." -ForegroundColor Yellow
Write-Host ""

# Run cargo check and filter for flashback/pool errors
$output = cargo check 2>&1 | Select-String -Pattern "(flashback|pool)" | Select-Object -First 50

if ($output) {
    Write-Host "Compilation messages for flashback/pool modules:" -ForegroundColor Yellow
    Write-Host ""
    $output | ForEach-Object { Write-Host $_ }
    Write-Host ""

    # Check if there are actual errors vs warnings
    $errors = $output | Select-String -Pattern "error\[E\d+\]"
    if ($errors) {
        Write-Host "⚠ There are still compilation errors in flashback/pool modules" -ForegroundColor Red
        Write-Host "Please review the errors above." -ForegroundColor Red
    } else {
        Write-Host "✓ No compilation errors found (only warnings if any)" -ForegroundColor Green
    }
} else {
    Write-Host "✓ No specific flashback/pool messages found" -ForegroundColor Green
    Write-Host "Running full cargo check to verify overall compilation status..." -ForegroundColor Yellow

    $fullCheck = cargo check 2>&1
    $fullErrors = $fullCheck | Select-String -Pattern "error\[E\d+\]"

    if ($fullErrors) {
        Write-Host ""
        Write-Host "Note: There are compilation errors in other modules." -ForegroundColor Yellow
        Write-Host "But flashback and pool modules appear to be fixed." -ForegroundColor Green
    } else {
        Write-Host ""
        Write-Host "✓ Full project compiles successfully!" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "=" * 80 -ForegroundColor Cyan
Write-Host "VERIFICATION COMPLETE" -ForegroundColor Cyan
Write-Host "=" * 80 -ForegroundColor Cyan

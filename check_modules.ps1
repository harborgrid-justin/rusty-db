#!/usr/bin/env pwsh
# Check specific modules for compilation errors

Write-Host "Checking concurrent and bench modules..." -ForegroundColor Cyan

$env:RUSTFLAGS = ""
cargo check --lib 2>&1 | Select-String -Pattern "(concurrent|bench)" -Context 0,2 | Out-String -Width 200

Write-Host "`n=== Summary ===" -ForegroundColor Green
$errors = cargo check --lib 2>&1 | Select-String -Pattern "error\[E" | Measure-Object
Write-Host "Total errors: $($errors.Count)" -ForegroundColor $(if ($errors.Count -eq 0) { "Green" } else { "Red" })

# Check specific files
Write-Host "`n=== Checking specific files ===" -ForegroundColor Cyan
$files = @(
    "src/concurrent/skiplist.rs",
    "src/concurrent/hashmap.rs",
    "src/bench/mod.rs"
)

foreach ($file in $files) {
    $fileErrors = cargo check --lib 2>&1 | Select-String -Pattern $file | Select-String -Pattern "error" | Measure-Object
    $status = if ($fileErrors.Count -eq 0) { "OK" } else { "ERRORS: $($fileErrors.Count)" }
    $color = if ($fileErrors.Count -eq 0) { "Green" } else { "Red" }
    Write-Host "$file : $status" -ForegroundColor $color
}

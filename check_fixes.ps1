# Check compilation of fixed files
Write-Host "Checking spatial/analysis.rs and multitenant/cloning.rs compilation..." -ForegroundColor Cyan

$ErrorActionPreference = "Continue"

# Run cargo check and filter for our specific files
$output = cargo check 2>&1 | Select-String -Pattern "(spatial/analysis|multitenant/cloning)" -Context 2,2

if ($output) {
    Write-Host "`nErrors found in target files:" -ForegroundColor Red
    $output | ForEach-Object { Write-Host $_.Line }
    exit 1
} else {
    Write-Host "`nNo errors found in spatial/analysis.rs or multitenant/cloning.rs!" -ForegroundColor Green

    # Check if there are any errors at all
    $allErrors = cargo check 2>&1 | Select-String -Pattern "^error"
    if ($allErrors) {
        Write-Host "`nNote: There are other compilation errors in the codebase, but not in our target files." -ForegroundColor Yellow
    } else {
        Write-Host "`nFull codebase compiles successfully!" -ForegroundColor Green
    }
    exit 0
}

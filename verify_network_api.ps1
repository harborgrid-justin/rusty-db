# Verification script for network and API modules
# Run this to verify that network and API modules compile without errors

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Network and API Module Verification" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "Checking for errors in network modules..." -ForegroundColor Yellow
$networkErrors = cargo check --lib 2>&1 | Select-String "src/network.*error"

if ($networkErrors) {
    Write-Host "❌ Errors found in network modules:" -ForegroundColor Red
    $networkErrors | ForEach-Object { Write-Host $_ -ForegroundColor Red }
} else {
    Write-Host "✅ No errors in network modules" -ForegroundColor Green
}

Write-Host ""
Write-Host "Checking for errors in API modules..." -ForegroundColor Yellow
$apiErrors = cargo check --lib 2>&1 | Select-String "src/api.*error"

if ($apiErrors) {
    Write-Host "❌ Errors found in API modules:" -ForegroundColor Red
    $apiErrors | ForEach-Object { Write-Host $_ -ForegroundColor Red }
} else {
    Write-Host "✅ No errors in API modules" -ForegroundColor Green
}

Write-Host ""
Write-Host "Checking for missing import errors..." -ForegroundColor Yellow
$importErrors = cargo check --lib 2>&1 | Select-String "(src/network|src/api)" | Select-String "cannot find (type|function|value)" | Select-String "(Mutex|sleep|interval)"

if ($importErrors) {
    Write-Host "❌ Missing imports detected:" -ForegroundColor Red
    $importErrors | ForEach-Object { Write-Host $_ -ForegroundColor Red }
} else {
    Write-Host "✅ All imports present" -ForegroundColor Green
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Verification Complete" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

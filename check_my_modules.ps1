# PowerShell script to check backup, flashback, and monitoring modules
# Run this to get compilation errors for Agent 7 modules

Write-Host "Checking RustyDB backup/, flashback/, and monitoring/ modules..."
Write-Host "============================================================"

cd F:\temp\rusty-db

# Run cargo check and filter for our modules
cargo check 2>&1 | Select-String -Pattern "(error|warning).*(backup/|flashback/|monitoring/)" |
    Out-File -FilePath "F:\temp\rusty-db\agent7_errors.txt"

# Also capture the full output
cargo check 2>&1 | Out-File -FilePath "F:\temp\rusty-db\full_check.txt"

Write-Host "`nErrors saved to: F:\temp\rusty-db\agent7_errors.txt"
Write-Host "Full output saved to: F:\temp\rusty-db\full_check.txt"

# Display filtered errors
Write-Host "`n=== ERRORS IN MY MODULES ===`n"
Get-Content "F:\temp\rusty-db\agent7_errors.txt"

# PowerShell script to show the structure of backup, flashback, and monitoring modules
# Displays file names, line counts, and key types exported

Write-Host "==================================================================" -ForegroundColor Cyan
Write-Host "  RustyDB Module Structure - backup/, flashback/, monitoring/     " -ForegroundColor Cyan
Write-Host "==================================================================" -ForegroundColor Cyan
Write-Host ""

function Show-ModuleInfo {
    param(
        [string]$ModulePath,
        [string]$ModuleName
    )

    Write-Host "[$ModuleName Module]" -ForegroundColor Yellow
    Write-Host ("=" * 60) -ForegroundColor Yellow

    $files = Get-ChildItem "$ModulePath/*.rs" | Sort-Object Name

    foreach ($file in $files) {
        $lineCount = (Get-Content $file.FullName | Measure-Object -Line).Lines
        $fileName = $file.Name

        Write-Host ("  {0,-30} {1,6} lines" -f $fileName, $lineCount) -ForegroundColor White

        # Show main structs/enums
        $mainTypes = Select-String -Path $file.FullName -Pattern "^pub (struct|enum) (\w+)" |
            ForEach-Object { $_.Matches.Groups[2].Value } |
            Select-Object -First 3

        if ($mainTypes) {
            Write-Host ("    Main types: {0}" -f ($mainTypes -join ", ")) -ForegroundColor Gray
        }
    }

    Write-Host ""
}

# Show each module
Show-ModuleInfo -ModulePath "F:\temp\rusty-db\src\backup" -ModuleName "BACKUP"
Show-ModuleInfo -ModulePath "F:\temp\rusty-db\src\flashback" -ModuleName "FLASHBACK"
Show-ModuleInfo -ModulePath "F:\temp\rusty-db\src\monitoring" -ModuleName "MONITORING"

# Total statistics
Write-Host "==================================================================" -ForegroundColor Cyan
Write-Host "  Statistics                                                      " -ForegroundColor Cyan
Write-Host "==================================================================" -ForegroundColor Cyan

$backupLines = (Get-ChildItem "F:\temp\rusty-db\src\backup/*.rs" | ForEach-Object {
    (Get-Content $_.FullName | Measure-Object -Line).Lines
} | Measure-Object -Sum).Sum

$flashbackLines = (Get-ChildItem "F:\temp\rusty-db\src\flashback/*.rs" | ForEach-Object {
    (Get-Content $_.FullName | Measure-Object -Line).Lines
} | Measure-Object -Sum).Sum

$monitoringLines = (Get-ChildItem "F:\temp\rusty-db\src\monitoring/*.rs" | ForEach-Object {
    (Get-Content $_.FullName | Measure-Object -Line).Lines
} | Measure-Object -Sum).Sum

$totalLines = $backupLines + $flashbackLines + $monitoringLines
$totalFiles = (Get-ChildItem "F:\temp\rusty-db\src\backup/*.rs").Count +
              (Get-ChildItem "F:\temp\rusty-db\src\flashback/*.rs").Count +
              (Get-ChildItem "F:\temp\rusty-db\src\monitoring/*.rs").Count

Write-Host ""
Write-Host ("  Backup module:      {0,6} lines in {1,2} files" -f $backupLines, (Get-ChildItem "F:\temp\rusty-db\src\backup/*.rs").Count) -ForegroundColor Green
Write-Host ("  Flashback module:   {0,6} lines in {1,2} files" -f $flashbackLines, (Get-ChildItem "F:\temp\rusty-db\src\flashback/*.rs").Count) -ForegroundColor Green
Write-Host ("  Monitoring module:  {0,6} lines in {1,2} files" -f $monitoringLines, (Get-ChildItem "F:\temp\rusty-db\src\monitoring/*.rs").Count) -ForegroundColor Green
Write-Host ("  " + ("-" * 45)) -ForegroundColor White
Write-Host ("  TOTAL:             {0,6} lines in {1,2} files" -f $totalLines, $totalFiles) -ForegroundColor Cyan
Write-Host ""

Write-Host "==================================================================" -ForegroundColor Cyan
Write-Host ""

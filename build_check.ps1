cargo build 2>&1 | Select-String -Pattern "error|backup/|flashback/|monitoring/" | Out-File -FilePath "F:\temp\rusty-db\build_errors.txt"

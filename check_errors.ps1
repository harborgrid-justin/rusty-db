cargo check 2>&1 | Select-String -Pattern "flashback|pool" | Select-Object -First 100

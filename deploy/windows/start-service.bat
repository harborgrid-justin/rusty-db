@echo off
REM ============================================================================
REM RustyDB Windows Service Start Script
REM ============================================================================

setlocal

set INSTANCE=%1
if "%INSTANCE%"=="" set INSTANCE=default
set SERVICE_NAME=RustyDB_%INSTANCE%

echo Starting %SERVICE_NAME%...
sc start %SERVICE_NAME%

if %errorLevel% equ 0 (
    echo Service started successfully.
) else (
    echo Failed to start service. Check Event Viewer for details.
)

endlocal

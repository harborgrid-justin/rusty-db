@echo off
REM ============================================================================
REM RustyDB Windows Service Stop Script
REM ============================================================================

setlocal

set INSTANCE=%1
if "%INSTANCE%"=="" set INSTANCE=default
set SERVICE_NAME=RustyDB_%INSTANCE%

echo Stopping %SERVICE_NAME%...
sc stop %SERVICE_NAME%

if %errorLevel% equ 0 (
    echo Service stopped successfully.
) else (
    echo Failed to stop service. It may not be running.
)

endlocal

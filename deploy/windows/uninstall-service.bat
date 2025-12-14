@echo off
REM ============================================================================
REM RustyDB Windows Service Uninstaller
REM ============================================================================
REM
REM Usage: uninstall-service.bat [instance_name]
REM
REM Examples:
REM   uninstall-service.bat           - Uninstall default instance
REM   uninstall-service.bat prod      - Uninstall 'prod' instance
REM
REM ============================================================================

setlocal enabledelayedexpansion

REM Check for admin rights
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo ERROR: This script requires administrator privileges.
    echo Please run as Administrator.
    exit /b 1
)

REM Get instance name
set INSTANCE=%1
if "%INSTANCE%"=="" set INSTANCE=default

set SERVICE_NAME=RustyDB_%INSTANCE%
set HOME=C:\ProgramData\RustyDB\instances\%INSTANCE%

echo.
echo ============================================================================
echo RustyDB Windows Service Uninstaller
echo ============================================================================
echo.
echo Instance Name: %INSTANCE%
echo Service Name:  %SERVICE_NAME%
echo.

REM Check if service exists
sc query %SERVICE_NAME% >nul 2>&1
if %errorLevel% neq 0 (
    echo Service %SERVICE_NAME% does not exist.
    exit /b 0
)

REM Confirm uninstall
set /p CONFIRM="Are you sure you want to uninstall %SERVICE_NAME%? (y/N): "
if /i "%CONFIRM%" neq "y" (
    echo Uninstallation cancelled.
    exit /b 0
)

REM Stop the service
echo Stopping service %SERVICE_NAME%...
sc stop %SERVICE_NAME% >nul 2>&1
timeout /t 3 /nobreak >nul

REM Delete the service
echo Removing service %SERVICE_NAME%...
sc delete %SERVICE_NAME%

if %errorLevel% neq 0 (
    echo ERROR: Failed to remove service. It may still be running.
    echo Try: taskkill /f /im rusty-db-server.exe
    exit /b 1
)

echo.
echo Service %SERVICE_NAME% has been removed.
echo.
echo NOTE: Instance data at %HOME% has NOT been deleted.
echo To remove data, manually delete: %HOME%
echo.

endlocal
exit /b 0

@echo off
REM ============================================================================
REM RustyDB Windows Service Installer
REM Instance Layout Spec v1.0
REM Version: 0.3.001
REM ============================================================================
REM
REM Usage: install-service.bat [instance_name]
REM
REM Examples:
REM   install-service.bat           - Install default instance
REM   install-service.bat prod      - Install 'prod' instance
REM   install-service.bat staging   - Install 'staging' instance
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

REM Get instance name (default: "default")
set INSTANCE=%1
if "%INSTANCE%"=="" set INSTANCE=default

REM Configuration paths
set BINARY="C:\Program Files\RustyDB\current\bin\rusty-db-server.exe"
set HOME=C:\ProgramData\RustyDB\instances\%INSTANCE%
set SERVICE_NAME=RustyDB_%INSTANCE%

echo.
echo ============================================================================
echo RustyDB Windows Service Installer
echo ============================================================================
echo.
echo Instance Name: %INSTANCE%
echo Service Name:  %SERVICE_NAME%
echo Binary:        %BINARY%
echo Home:          %HOME%
echo.

REM Check if binary exists
if not exist %BINARY% (
    echo ERROR: Binary not found at %BINARY%
    echo Please install RustyDB first.
    exit /b 1
)

REM Create instance directory if it doesn't exist
if not exist "%HOME%" (
    echo Creating instance directory: %HOME%
    mkdir "%HOME%"
    mkdir "%HOME%\conf"
    mkdir "%HOME%\data"
    mkdir "%HOME%\logs"
    mkdir "%HOME%\run"
    mkdir "%HOME%\cache"
    mkdir "%HOME%\tmp"
    mkdir "%HOME%\backup"
    mkdir "%HOME%\diag"

    REM Copy default configuration if available
    if exist "C:\Program Files\RustyDB\current\conf\rustydb.toml" (
        copy "C:\Program Files\RustyDB\current\conf\rustydb.toml" "%HOME%\conf\"
        echo Copied default configuration.
    )
)

REM Check if service already exists
sc query %SERVICE_NAME% >nul 2>&1
if %errorLevel% equ 0 (
    echo WARNING: Service %SERVICE_NAME% already exists.
    set /p CONFIRM="Do you want to reinstall? (y/N): "
    if /i "!CONFIRM!" neq "y" (
        echo Installation cancelled.
        exit /b 0
    )

    echo Stopping existing service...
    sc stop %SERVICE_NAME% >nul 2>&1
    timeout /t 2 /nobreak >nul

    echo Removing existing service...
    sc delete %SERVICE_NAME%
    timeout /t 2 /nobreak >nul
)

REM Create the service
echo Creating service %SERVICE_NAME%...
sc.exe create %SERVICE_NAME% ^
    binPath= "%BINARY% --home \"%HOME%\"" ^
    start= auto ^
    DisplayName= "RustyDB (%INSTANCE%)"

if %errorLevel% neq 0 (
    echo ERROR: Failed to create service.
    exit /b 1
)

REM Set service description
sc.exe description %SERVICE_NAME% "RustyDB database instance '%INSTANCE%' (Instance Layout Spec v1.0, Version 0.3.001)"

REM Configure recovery options (restart on failure)
sc.exe failure %SERVICE_NAME% reset= 86400 actions= restart/2000/restart/5000/restart/10000

REM Configure delayed auto-start for better boot performance
sc.exe config %SERVICE_NAME% start= delayed-auto

echo.
echo ============================================================================
echo Service %SERVICE_NAME% installed successfully!
echo ============================================================================
echo.
echo To start the service:
echo   sc start %SERVICE_NAME%
echo   -- or --
echo   net start %SERVICE_NAME%
echo.
echo To view logs:
echo   type "%HOME%\logs\rustydb.log"
echo.
echo Configuration file:
echo   %HOME%\conf\rustydb.toml
echo.

endlocal
exit /b 0

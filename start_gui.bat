@echo off
setlocal
cd /d "%~dp0"
echo === Agent Manager GUI ===
echo.

where python >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Python not found in PATH.
    echo Please install Python 3.10+ from https://www.python.org/downloads/
    pause
    exit /b 1
)

set PYTHONIOENCODING=utf-8
python -m app.main
if errorlevel 1 (
    echo.
    echo [ERROR] GUI exited with error.
    pause
)
endlocal

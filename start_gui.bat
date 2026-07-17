@echo off
setlocal
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

cargo run --release --
if errorlevel 1 (
  echo.
  echo Agent Manager 啟動失敗。請確認已安裝 Rust stable toolchain。
  pause
)

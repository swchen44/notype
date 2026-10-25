@echo off
setlocal DisableDelayedExpansion
if not exist "%~dp0notype-deepseek-probe.exe" (
  echo The reviewed helper binary is not present. This source launcher is not ready for use.
  exit /b 1
)
start "" "%~dp0notype-deepseek-probe.exe"

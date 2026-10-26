@echo off
setlocal DisableDelayedExpansion
if not "%~2"=="" exit /b 1
if "%~1"=="check" goto check
if "%~1"=="run" goto run
if "%~1"=="report" goto report
echo probe-arguments-unavailable 1>&2
exit /b 1
:check
set "probeMode=check"
goto allowed
:run
set "probeMode=run"
goto allowed
:report
set "probeMode=report"
:allowed
if not exist "%~dp0notype-provider-probe.exe" (
  echo probe-artifact-unavailable 1>&2
  exit /b 1
)
"%~dp0notype-provider-probe.exe" %probeMode%
exit /b %errorlevel%

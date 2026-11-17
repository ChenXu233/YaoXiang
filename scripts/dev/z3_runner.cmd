@echo off
rem cargo runner wrapper (Windows): .cargo/config.toml points the runner at this file.
rem Why a wrapper: a runner string that names an interpreter (python/python3/py) resolves
rem that name through PATH -- gate hosts may invoke gates with a stripped PATH that lacks
rem every Python directory (measured: cargo reports program not found, the snapshot gate
rem dies before running any test). This wrapper probes interpreters at fixed locations
rem first, so the runner only needs cmd.exe and %SystemRoot%:
rem   1) %SystemRoot%\py.exe           (official installer, all-users launcher)
rem   2) %LOCALAPPDATA%\Programs\Python\Launcher\py.exe (per-user launcher)
rem   3) python / python3 on PATH       (conda and other managed environments)
rem The runner script itself is located via %~dp0, independent of the caller's cwd.
setlocal
set "PY_SCRIPT=%~dp0z3_runner.py"
if exist "%SystemRoot%\py.exe" (
    "%SystemRoot%\py.exe" -3 "%PY_SCRIPT%" %*
    exit /b %ERRORLEVEL%
)
if exist "%LOCALAPPDATA%\Programs\Python\Launcher\py.exe" (
    "%LOCALAPPDATA%\Programs\Python\Launcher\py.exe" -3 "%PY_SCRIPT%" %*
    exit /b %ERRORLEVEL%
)
where python >nul 2>nul
if not errorlevel 1 (
    python "%PY_SCRIPT%" %*
    exit /b %ERRORLEVEL%
)
where python3 >nul 2>nul
if not errorlevel 1 (
    python3 "%PY_SCRIPT%" %*
    exit /b %ERRORLEVEL%
)
echo z3_runner.cmd: no Python interpreter found (tried py/python/python3) 1>&2
exit /b 127

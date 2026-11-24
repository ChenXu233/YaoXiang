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
rem
rem Exit-code propagation: NO parenthesized `if (...)` blocks anywhere in this file.
rem `%ERRORLEVEL%` inside a block is expanded when the whole `if` statement is parsed,
rem that is, before the interpreter runs -- the child's real exit code is replaced by
rem the pre-block value (measured 2026-11-19: child exits 0/1/2/5/101/127 all came
rem back flattened through every probe branch, and cargo saw exit 0 for a libtest
rem that exited 101, so run-time failures leaked past exit-code gates). Each
rem interpreter call is a bare statement; every branch gotos the single top-level
rem `exit /b %ERRORLEVEL%` below, which is parsed at execution time.
setlocal
set "PY_SCRIPT=%~dp0z3_runner.py"

if exist "%SystemRoot%\py.exe" goto py_systemroot
if exist "%LOCALAPPDATA%\Programs\Python\Launcher\py.exe" goto py_userlauncher
where python >nul 2>nul
if not errorlevel 1 goto py_path_python
where python3 >nul 2>nul
if not errorlevel 1 goto py_path_python3
echo z3_runner.cmd: no Python interpreter found (tried py/python/python3) 1>&2
exit /b 127

:py_systemroot
"%SystemRoot%\py.exe" -3 "%PY_SCRIPT%" %*
goto propagate

:py_userlauncher
"%LOCALAPPDATA%\Programs\Python\Launcher\py.exe" -3 "%PY_SCRIPT%" %*
goto propagate

:py_path_python
python "%PY_SCRIPT%" %*
goto propagate

:py_path_python3
python3 "%PY_SCRIPT%" %*
goto propagate

:propagate
exit /b %ERRORLEVEL%

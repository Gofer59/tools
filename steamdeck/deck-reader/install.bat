@echo off
REM One-liner wrapper for install.ps1 — bypasses execution policy for the current process only.
powershell -ExecutionPolicy Bypass -File "%~dp0install.ps1" %*

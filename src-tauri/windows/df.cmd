@setlocal DisableDelayedExpansion
@if "%~1"=="" (
    start "" "%~dp0..\devfleet.exe" --add-nearest-project
) else if /I "%~1"=="scan" (
    start "" "%~dp0..\devfleet.exe" --scan-projects
) else (
    start "" "%~dp0..\devfleet.exe" --add-project "%~1"
)

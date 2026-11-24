@setlocal DisableDelayedExpansion
@if "%~1"=="" (
    start "" "%~dp0..\devfleet.exe" --add-nearest-project
) else (
    start "" "%~dp0..\devfleet.exe" --add-project "%~1"
)

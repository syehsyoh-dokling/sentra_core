$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "Starting Core Worker..." -ForegroundColor Cyan
cargo run -p core-worker

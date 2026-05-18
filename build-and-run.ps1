$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "=== BUILD CHECK ===" -ForegroundColor Cyan
cargo build

Write-Host "=== START INFRA ===" -ForegroundColor Cyan
docker compose up -d postgres redis

Write-Host "=== QUICK API RUN ===" -ForegroundColor Cyan
cargo run -p core-api

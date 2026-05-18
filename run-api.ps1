$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "Starting PostgreSQL and Redis..." -ForegroundColor Cyan
docker compose up -d postgres redis

Write-Host "Waiting for services..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

Write-Host "Building Rust workspace..." -ForegroundColor Cyan
cargo build

Write-Host "Starting Core API at http://127.0.0.1:4000" -ForegroundColor Green
cargo run -p core-api

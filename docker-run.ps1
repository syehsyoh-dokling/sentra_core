$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "=== START SENTRACORE CORE BACKEND VIA DOCKER ===" -ForegroundColor Cyan

docker compose down
docker compose up -d --build

Write-Host "`n=== CONTAINERS ===" -ForegroundColor Cyan
docker ps --filter "name=sentracore_core"

Write-Host "`n=== API LOGS LAST 80 LINES ===" -ForegroundColor Cyan
docker logs sentracore_core_api --tail 80

Write-Host "`nHealth check:" -ForegroundColor Cyan
Write-Host "Invoke-RestMethod -Method GET -Uri http://127.0.0.1:4000/health" -ForegroundColor White

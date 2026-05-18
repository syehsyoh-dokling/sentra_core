Write-Host "=== SENTRACORE CORE CONTAINERS ===" -ForegroundColor Cyan
docker ps --filter "name=sentracore_core"

Write-Host "`n=== API LOGS ===" -ForegroundColor Cyan
docker logs sentracore_core_api --tail 120

Write-Host "`n=== WORKER LOGS ===" -ForegroundColor Cyan
docker logs sentracore_core_worker --tail 120

Write-Host "`n=== POSTGRES LOGS ===" -ForegroundColor Cyan
docker logs sentracore_core_postgres --tail 80

Write-Host "`n=== REDIS LOGS ===" -ForegroundColor Cyan
docker logs sentracore_core_redis --tail 80

$ErrorActionPreference = "Stop"

$Base = "http://127.0.0.1:4000"

Write-Host "=== APP1 EXTERNAL PROVIDERS ===" -ForegroundColor Cyan
Invoke-RestMethod -Method GET -Uri "$Base/api/v1/external/providers"

Write-Host "`n=== APP1 SYSTEM HEALTH ===" -ForegroundColor Cyan
Invoke-RestMethod -Method GET -Uri "$Base/api/v1/admin/system-health"

Write-Host "`n=== LOGIN ===" -ForegroundColor Cyan
$login = Invoke-RestMethod -Method POST -Uri "$Base/api/v1/auth/login" -ContentType "application/json" -Body (@{
  email = "saifuddin@example.com"
  password = "password123"
} | ConvertTo-Json)

$token = $login.data.token
$headers = @{ Authorization = "Bearer $token" }

Write-Host "`n=== CREATE KYC CASE ===" -ForegroundColor Cyan
$kyc = Invoke-RestMethod -Method POST -Uri "$Base/api/v1/kyc/start" -ContentType "application/json" -Body (@{
  provider = "sumsub-sandbox-mock"
} | ConvertTo-Json)
$kyc

Write-Host "`n=== CREATE SIGNED UPLOAD ===" -ForegroundColor Cyan
$upload = Invoke-RestMethod -Method POST -Uri "$Base/api/v1/uploads/signed-url" -ContentType "application/json" -Body (@{
  file_name = "Staking.sol"
  file_type = "solidity"
} | ConvertTo-Json)
$upload

Write-Host "`n=== CREATE AUDIT REQUEST ===" -ForegroundColor Cyan
$audit = Invoke-RestMethod -Method POST -Uri "$Base/api/v1/audits" -ContentType "application/json" -Body (@{
  blockchain = "POLYGON"
  priority = "HIGH"
  audit_type = "FULL"
  notes = "Please focus on staking logic"
  contract_ids = @()
} | ConvertTo-Json -Depth 10)
$auditId = $audit.data.id
$audit

Write-Host "`n=== CREATE PAYMENT MOCK ===" -ForegroundColor Cyan
$payment = Invoke-RestMethod -Method POST -Uri "$Base/api/v1/payments/create" -ContentType "application/json" -Body (@{
  audit_id = $auditId
  amount_idr = 2500000
  provider = "midtrans-sandbox-mock"
} | ConvertTo-Json)
$payment

Write-Host "`n=== VALIDATE AUDIT ===" -ForegroundColor Cyan
Invoke-RestMethod -Method POST -Uri "$Base/api/v1/audits/$auditId/validate"

Write-Host "`n=== CREATE AUDIT JOB ===" -ForegroundColor Cyan
$job = Invoke-RestMethod -Method POST -Uri "$Base/api/v1/audit-jobs" -ContentType "application/json" -Body (@{
  audit_id = $auditId
  priority = "HIGH"
} | ConvertTo-Json)
$jobId = $job.data.id
$job

Write-Host "`n=== TRANSFER TO APP2 MOCK/EXTERNAL ===" -ForegroundColor Cyan
Invoke-RestMethod -Method POST -Uri "$Base/internal/v1/app2/audit-jobs" -ContentType "application/json" -Body (@{
  job_id = $jobId
  audit_id = $auditId
  blockchain = "POLYGON"
  compiler_version = "0.8.20"
  priority = "HIGH"
  contracts = @(
    @{
      path = "contracts/Staking.sol"
      storage_url = $upload.data.storage_url
    }
  )
} | ConvertTo-Json -Depth 10)

Write-Host "`n=== AUDIT STATUS ===" -ForegroundColor Cyan
Invoke-RestMethod -Method GET -Uri "$Base/api/v1/audits/$auditId/status"

Write-Host "`n=== GENERIC EXTERNAL API TEST: HTTPBIN ===" -ForegroundColor Cyan
Invoke-RestMethod -Method POST -Uri "$Base/api/v1/external/test" -ContentType "application/json" -Body (@{
  provider = "httpbin"
  method = "POST"
  url = "https://httpbin.org/post"
  body = @{
    app = "app1"
    purpose = "external-api-connectivity-test"
  }
} | ConvertTo-Json -Depth 10)

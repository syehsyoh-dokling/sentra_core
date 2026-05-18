# Health
Invoke-RestMethod -Method GET -Uri "http://127.0.0.1:4000/health"

# Register
Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:4000/auth/register" -ContentType "application/json" -Body (@{
  name = "Saifuddin"
  email = "saifuddin@example.com"
  password = "password123"
} | ConvertTo-Json)

# Login
$login = Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:4000/auth/login" -ContentType "application/json" -Body (@{
  email = "saifuddin@example.com"
  password = "password123"
} | ConvertTo-Json)

$token = $login.data.token
$headers = @{ Authorization = "Bearer $token" }

# Me
Invoke-RestMethod -Method GET -Uri "http://127.0.0.1:4000/auth/me" -Headers $headers

# Organization
$org = Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:4000/organizations" -ContentType "application/json" -Body (@{
  name = "Demo Web3 Studio"
  slug = "demo-web3-studio"
} | ConvertTo-Json)

$orgId = $org.data.id

# Project
$project = Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:4000/projects" -ContentType "application/json" -Body (@{
  organization_id = $orgId
  name = "Demo ERC20 Launch"
  description = "Local smart contract core backend test"
} | ConvertTo-Json)

$projectId = $project.data.id

# Chain
$chain = Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:4000/chains" -ContentType "application/json" -Body (@{
  name = "Ethereum Sepolia"
  chain_type = "evm"
  chain_id = 11155111
  rpc_url = "https://sepolia.example-rpc.local"
  explorer_url = "https://sepolia.etherscan.io"
  native_symbol = "ETH"
  is_testnet = $true
} | ConvertTo-Json)

$chainId = $chain.data.id

# Wallet
$wallet = Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:4000/wallets" -ContentType "application/json" -Body (@{
  organization_id = $orgId
  address = "0x1234567890abcdef1234567890abcdef12345678"
  chain_type = "evm"
  label = "Local Deployer Wallet"
} | ConvertTo-Json)

$walletId = $wallet.data.id

# Contract
$contract = Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:4000/contracts" -ContentType "application/json" -Body (@{
  project_id = $projectId
  chain_id = $chainId
  name = "DemoToken"
  language = "solidity"
  framework = "hardhat"
  compiler_version = "0.8.20"
} | ConvertTo-Json)

$contractId = $contract.data.id

# Deployment, this queues a Redis job for worker
$deployment = Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:4000/deployments" -ContentType "application/json" -Body (@{
  contract_id = $contractId
  chain_id = $chainId
  deployer_wallet_id = $walletId
  deployer_address = "0x1234567890abcdef1234567890abcdef12345678"
  contract_address = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
  tx_hash = "0x9876543210abcdef9876543210abcdef9876543210abcdef9876543210abcd"
} | ConvertTo-Json)

# Lists
Invoke-RestMethod -Method GET -Uri "http://127.0.0.1:4000/list/projects"
Invoke-RestMethod -Method GET -Uri "http://127.0.0.1:4000/list/contracts"
Invoke-RestMethod -Method GET -Uri "http://127.0.0.1:4000/list/deployments"
Invoke-RestMethod -Method GET -Uri "http://127.0.0.1:4000/list/jobs"

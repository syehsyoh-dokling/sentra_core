# Sentra Core

Sentra Core backend for Sentracore.

Responsibilities:
- auth and user/project administration
- audit intake and validation
- audit job gateway
- payment/mock payment flow
- queue handoff to SentraGuard

## Local Run

```bash
docker compose up -d --build
```

Core API:

```text
http://127.0.0.1:4000
```

Frontend:

```text
http://localhost:4100
```

## CI/CD

GitHub Actions workflow is in `.github/workflows/ci-cd.yml`.

Required deployment secrets are documented in that workflow.

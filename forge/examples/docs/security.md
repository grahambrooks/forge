The Payment Platform operates within PCI DSS Level 1 scope. This document outlines the security architecture and controls.

## PCI DSS Scope

The following components are in PCI scope (they process, store, or transmit cardholder data):

- **Payment API** -- Receives card numbers from customers, tokenizes immediately
- **Payment Processor** -- Handles tokenized card data for authorization
- **Ledger DB** -- Stores transaction records (tokens only, never raw PANs)

Out of PCI scope:
- **Notification Service** -- Only receives transaction IDs, not card data
- **Session Cache** -- Stores session tokens and idempotency keys, no CHD

## Authentication & Authorization

### External (Customer-facing)

- OAuth 2.0 + PKCE for web/mobile clients
- API keys with HMAC signatures for merchant integrations
- All external traffic over TLS 1.3

### Internal (Service-to-service)

- mTLS between all services within the Kubernetes cluster
- gRPC calls authenticated via service mesh (Istio)
- Database access via IAM-authenticated connection pooling (PgBouncer)

## Data Protection

### Encryption

| Layer | Method | Key Management |
|-------|--------|---------------|
| In transit | TLS 1.3 (external), mTLS (internal) | AWS ACM |
| At rest (DB) | AES-256-GCM | AWS KMS |
| At rest (backups) | AES-256 | AWS KMS (separate key) |
| Card data | Tokenization via Stripe/Adyen | PSP-managed |

### Data Retention

- Transaction records: 7 years (regulatory requirement)
- Session data: 24-hour TTL
- Logs: 90 days in CloudWatch, archived to S3 Glacier
- Audit trail: 7 years (immutable append-only table)

## Security Scanning

The CI/CD pipeline includes automated security gates:

1. **Dependency audit** -- `cargo audit` checks for known vulnerabilities in Rust crates
2. **Container scanning** -- Trivy scans for OS and library CVEs
3. **Secret detection** -- Gitleaks prevents accidental credential commits
4. **SAST** -- Static analysis for common vulnerability patterns

Any critical or high finding blocks the pipeline. Medium findings generate warnings and must be addressed within 30 days.

## Incident Response

- **Severity 1** (data breach, service down): 15-minute response SLA, war room activated
- **Severity 2** (degraded performance, failed payments): 1-hour response SLA
- **Severity 3** (non-critical security finding): 24-hour triage, 30-day remediation

All security incidents are logged in the audit trail and reported per PCI DSS requirements.

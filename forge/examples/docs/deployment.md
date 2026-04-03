The Payment Platform runs on Kubernetes across three environments with progressive promotion through the CI/CD pipeline.

## Environments

| Environment | Purpose | Cluster | Data |
|-------------|---------|---------|------|
| **Development** | Local testing | Docker Compose | Seeded test data |
| **Staging** | Integration testing, QA | `payments-staging` (us-east-1) | Anonymized production snapshot |
| **Production** | Live traffic | `payments-prod` (us-east-1, eu-west-1) | Real customer data (PCI scope) |

## Deployment Pipeline

Changes flow through four stages:

1. **Build & Test** -- Compiles the Rust workspace, runs unit and integration tests, generates code coverage reports
2. **Security Scan** -- Trivy scans the container image for CVEs; blocked on any critical or high vulnerabilities
3. **Deploy Staging** -- Automated deploy to staging; integration tests run against the staging environment
4. **Deploy Production** -- Requires manual approval from `platform-team`; canary rollout over 30 minutes

## Infrastructure

See the **[Production Deployment](../views/Deployment.html)** diagram for the full deployment topology. The key infrastructure components are:

## Scaling Strategy

- **Payment API**: Horizontal scaling based on CPU/request rate. Target: 3-9 replicas per region.
- **Payment Processor**: Scales with API; runs as a library within the API pods.
- **Ledger DB**: Vertical scaling for writes, read replicas for reporting queries.
- **Redis**: 3-node cluster with consistent hashing. Scales by adding shards.
- **Notification Service**: Scales based on queue depth. Tolerant of brief delays.

## Disaster Recovery

- **RPO** (Recovery Point Objective): 0 seconds (synchronous replication)
- **RTO** (Recovery Time Objective): < 5 minutes (automated failover)
- **Backup**: Daily full backups to S3 with point-in-time recovery. 90-day retention.
- **Multi-region**: Active-passive with DNS failover. Manual promotion for write traffic.

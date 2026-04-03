## ADR-006: Add Notification Service and Session Cache

**Status**: Accepted
**Date**: 2026-04-02

### Context

The Payment Platform currently handles receipt notifications synchronously within the Payment Processor. This creates two problems:

1. **Latency impact**: Sending email and SMS receipts adds 200-400ms to payment authorization time. At peak load (10k TPS), this pushes p95 latency above our 200ms SLA.

2. **Session management**: Payment sessions and idempotency keys are stored in PostgreSQL alongside financial data. This mixes hot, ephemeral data with cold, durable data, causing unnecessary write amplification and connection pool pressure on the Ledger DB.

### Decision

**Add two new containers to the Payment Service:**

1. **Notification Service** (Go) — A standalone service that consumes payment events from an async message queue and handles email/SMS delivery independently. Go was chosen for its lightweight concurrency model and mature email/SMS library ecosystem.

2. **Session Cache** (Redis) — An in-memory cache for payment sessions, idempotency keys, and rate limiting. Redis provides sub-millisecond reads and TTL-based automatic expiry.

**New relationships:**
- Payment Processor sends receipt events to Notification Service via async message queue
- Payment API reads sessions from Session Cache via Redis protocol

### Consequences

**Positive:**
- Payment authorization latency reduced by ~300ms (receipt sending no longer on critical path)
- Ledger DB connection pool freed from session read/write pressure
- Notification failures no longer block payment processing
- Redis TTL handles session cleanup automatically (no batch jobs needed)

**Negative:**
- Two additional services to deploy, monitor, and maintain
- Async notification delivery means receipts may be delayed by seconds under load
- Redis is an additional point of failure for session reads (mitigated by graceful fallback to direct DB query)
- Need to ensure at-least-once delivery semantics for receipt notifications

### Deployment Impact

- EKS cluster requires 2 additional pod deployments (Notification Service x2 replicas)
- New ElastiCache Redis cluster (3-node) provisioned in us-east-1
- Updated deployment view reflects new infrastructure topology

Key architectural decisions for the Payment Platform, recorded as lightweight ADRs.

## ADR-001: Rust for Core Payment Processing

**Status**: Accepted

**Context**: The payment processor handles sensitive financial data at high throughput. We need predictable latency (no GC pauses), memory safety (no buffer overflows in PCI scope), and efficient concurrency.

**Decision**: Implement the Payment API and Payment Processor in Rust using Actix-web for HTTP and Tonic for gRPC.

**Consequences**:
- Predictable sub-millisecond GC-free latency
- Memory safety without runtime overhead
- Steeper learning curve for new team members
- Smaller ecosystem for financial libraries compared to Java/Go

## ADR-002: PostgreSQL as the Ledger Database

**Status**: Accepted

**Context**: Financial transactions require ACID guarantees, strong consistency, and auditability. We need a database that supports complex queries for reconciliation while handling write-heavy payment flows.

**Decision**: Use PostgreSQL 16 with synchronous replication for the ledger.

**Consequences**:
- Full ACID compliance for financial integrity
- Rich query capabilities for reconciliation reports
- Synchronous replication ensures zero data loss
- Higher write latency compared to eventual-consistency stores
- Requires careful connection pool management at scale

## ADR-003: Redis for Session Caching

**Status**: Accepted

**Context**: Payment sessions (cart state, idempotency keys, rate limiting) need sub-millisecond access but don't require durability.

**Decision**: Use Redis for session and idempotency key caching with TTL-based expiry.

**Consequences**:
- Sub-millisecond reads for hot payment sessions
- Automatic cleanup via TTL (no manual garbage collection)
- Data loss on Redis restart is acceptable (sessions are recreatable)
- Need to handle cache misses gracefully in the payment flow

## ADR-004: Async Notifications via Message Queue

**Status**: Accepted

**Context**: Sending email/SMS receipts synchronously in the payment flow would add latency and create a coupling between payment processing and notification delivery.

**Decision**: The Payment Processor sends receipt events to an async message queue. The Notification Service consumes these independently.

**Consequences**:
- Payment authorization latency unaffected by notification delivery
- Notification failures don't block payments
- At-least-once delivery semantics require idempotent notification handling
- Added operational complexity (message queue infrastructure)

## ADR-005: Trunk-Based Development with CI Gates

**Status**: Accepted

**Context**: With a small, experienced team working on PCI-scoped code, we need fast iteration with strong quality gates.

**Decision**: Use trunk-based development with short-lived feature branches. All merges to `main` require passing CI (build, test, security scan) and code review. Production deploys require manual approval.

**Consequences**:
- Fast integration cycle (no long-lived branches)
- Every commit to main is potentially deployable
- Security scanning catches vulnerabilities before merge
- Manual production gate provides human oversight for financial system changes

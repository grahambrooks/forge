# Payment Platform

Architecture documentation for the Payment Platform.

## System Context

{{ forge_view "SystemContext" }}

The Payment Platform processes card and bank payments. Customers interact with the system via HTTPS through the Payment API.

## Key Decisions

- **Rust** for the core API and processor — predictable latency, memory safety
- **PostgreSQL** for the ledger — ACID guarantees for financial transactions
- **gRPC** for internal service communication — typed contracts, efficient serialization

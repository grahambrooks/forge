# Container View

{{ forge_view "Containers" }}

## Payment API

REST + gRPC gateway handling all external requests. Built with Actix-web.

## Payment Processor

Core business logic for payment authorization, capture, and settlement.

## Ledger DB

PostgreSQL 16 with synchronous replication. Stores all transaction records.

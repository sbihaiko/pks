# API Design Best Practices

## Overview

An API is a contract between a provider and its consumers. Good API design minimizes breaking changes, reduces cognitive load for callers, and makes the happy path obvious. APIs are harder to change than internal code because consumers outside the team depend on them.

## RESTful Resource Modeling

Resources are nouns, not verbs. Collections use plural names: `/users`, `/orders`. A single resource is addressed with its identifier: `/users/42`. Actions that do not map cleanly to CRUD use sub-resources or purpose-built endpoints, but these should be rare.

### HTTP Semantics

GET is safe and idempotent — it never modifies state. PUT replaces a resource entirely and is idempotent. PATCH applies a partial update. POST creates or triggers a non-idempotent action. DELETE removes a resource. Correct use of HTTP verbs allows caches, proxies, and clients to apply standard optimizations.

### Status Codes

200 for success, 201 for resource creation with a Location header, 204 for success with no response body, 400 for client validation errors, 401 for unauthenticated requests, 403 for unauthorized requests, 404 for missing resources, 409 for conflicts, 422 for semantic validation failures, 429 for rate limiting, 500 for server errors. Status codes communicate intent independently of response bodies.

## Versioning Strategy

URL versioning (`/v1/`, `/v2/`) is visible and cacheable. Header versioning keeps URLs clean but is less discoverable. Avoiding breaking changes through additive evolution — new optional fields, new endpoints — is always preferable to version increments.

## Pagination and Filtering

Cursor-based pagination is more reliable than offset-based for large, changing datasets. Provide a `next_cursor` field in responses. Filtering through query parameters (`?status=active&sort=created_at`) keeps the resource URL stable while allowing flexible querying.

## Documentation and Contracts

OpenAPI (formerly Swagger) specifications serve as machine-readable contracts. Consumer-driven contract testing verifies that providers honor what consumers expect.

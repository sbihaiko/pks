# Software Architecture Principles

## Overview

Software architecture defines the high-level structure of a system — the components, their relationships, and the principles that guide their design and evolution. Good architecture reduces accidental complexity and makes change cheaper over time.

## Core Architectural Styles

### Layered Architecture

Layered systems separate concerns by grouping related responsibilities into horizontal tiers: presentation, application logic, domain, and infrastructure. Each layer depends only on the layer directly below it. This enforces a clean dependency direction and isolates volatile infrastructure details from stable domain logic.

### Hexagonal Architecture

Also called ports-and-adapters, hexagonal architecture places the domain model at the center. External systems — databases, HTTP clients, message queues — connect through well-defined interfaces called ports. Adapters implement those interfaces for specific technologies. The domain never imports infrastructure code.

### Event-Driven Architecture

Components communicate through events rather than direct calls. A producer emits an event when something happens; consumers react asynchronously. This decouples producers from consumers and scales well because consumers can be added without modifying producers. The trade-off is eventual consistency and harder-to-trace execution paths.

## Design Principles

### Single Responsibility

Every module, class, or function should have exactly one reason to change. When a component handles multiple concerns, changes to one concern risk breaking others.

### Dependency Inversion

High-level policy modules should not depend on low-level detail modules. Both should depend on abstractions. This allows swapping implementations without touching business logic.

### Open/Closed Principle

Software entities should be open for extension but closed for modification. New behavior is added by introducing new types that implement existing interfaces, not by editing existing code.

## Documentation and Fitness Functions

Architecture decisions should be recorded as Architecture Decision Records (ADRs). Fitness functions — automated checks that verify architectural properties — prevent drift. Examples include enforcing that no service imports from another service's internal packages, or that response latency stays below a threshold.

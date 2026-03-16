# Observability in Distributed Systems

## Overview

Observability is the ability to understand the internal state of a system from its external outputs. In distributed systems, where a single user request may traverse dozens of services, observability is not optional — it is the primary tool for diagnosing failures and understanding performance.

## The Three Pillars

### Logs

Logs are timestamped records of discrete events. Structured logs — JSON objects with consistent fields — are machine-parseable and support efficient filtering. Every log entry should carry a correlation ID that links it to the originating request. Log levels (DEBUG, INFO, WARN, ERROR) allow operators to tune verbosity without code changes.

### Metrics

Metrics are numeric measurements aggregated over time. Counters monotonically increase and measure totals such as request counts. Gauges record current values such as queue depths. Histograms bucket observations to compute percentile latencies. The USE method (Utilization, Saturation, Errors) and RED method (Rate, Errors, Duration) organize metrics into actionable dashboards.

### Distributed Tracing

Traces capture the causal chain of operations across service boundaries. A trace is a tree of spans; each span records a unit of work with its start time, duration, and attributes. Sampling reduces storage costs for high-traffic services. OpenTelemetry provides a vendor-neutral API and SDK for generating traces, metrics, and logs.

## Instrumentation Strategy

Automatic instrumentation captures HTTP middleware, database calls, and external RPC. Manual instrumentation adds business-level spans around domain operations. Both should propagate trace context — a trace ID and span ID — through HTTP headers and message queue metadata.

## Alerting

Alerts fire on symptoms, not causes. Alert on elevated error rate or degraded latency; let dashboards and traces reveal the cause. Page on conditions that require immediate human action; log everything else for asynchronous review. Alert fatigue from noisy, non-actionable alerts is a leading cause of operator burnout.

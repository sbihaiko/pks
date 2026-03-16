# Managing Technical Debt

## Overview

Technical debt is the accumulated cost of shortcuts taken during software development. Like financial debt, it carries interest: every future change must work around or compensate for the shortcut. Unmanaged, technical debt slows development, increases defect rates, and makes the system fragile. Managed deliberately, some debt is a rational trade-off.

## Categories of Technical Debt

### Deliberate Debt

A team that consciously chooses a quick solution to meet a deadline and records the trade-off is taking deliberate debt. The key is the recording: a TODO comment, a ticket, or an Architecture Decision Record that captures the shortcut and commits to addressing it.

### Accidental Debt

Accidental debt accumulates without awareness. Outdated dependencies, unused code paths, inconsistent naming conventions, and copy-pasted logic that diverges over time are common examples. Regular code audits, dependency update processes, and static analysis tools surface accidental debt before it compounds.

### Bit Rot

Systems that are not actively maintained decay even without change. External dependencies release breaking changes. Security vulnerabilities are discovered. Language features and idioms evolve. Staying current requires continuous investment in maintenance tasks that deliver no user-visible features.

## Strategies for Repayment

### The Boy Scout Rule

Leave code cleaner than you found it. Each time a developer touches an area of the codebase, small improvements — renaming, extracting, simplifying — reduce debt incrementally. This distributes the work of debt repayment across routine development.

### Dedicated Refactoring Sprints

Some debt is too large for incremental work. Periodic sprints focused exclusively on refactoring address structural problems that require coordinated, focused effort. These are most effective when the business understands the value in terms of future development velocity.

## Measuring Debt

Cyclomatic complexity, coupling metrics, and code churn (files changed frequently and in coordination) are proxies for debt concentration. Prioritize paying down debt in high-churn, high-complexity modules; they impose the greatest ongoing tax.

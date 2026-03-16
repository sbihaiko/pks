# Refactoring Techniques

## Overview

Refactoring is the process of restructuring existing code without changing its external behavior. It improves readability, reduces complexity, and prepares the codebase for future changes. Disciplined refactoring is done in small, tested steps with continuous verification that behavior is preserved.

## Foundational Techniques

### Extract Method

When a block of code inside a function can be grouped and named independently, extract it into its own function. This reduces function length, eliminates duplicated logic, and makes the parent function read at a consistent level of abstraction.

### Rename Symbol

Renaming variables, functions, types, and modules is the highest-ROI refactoring. A name that accurately describes what something is or does removes the need for comments and reduces the cognitive load of reading code.

### Replace Magic Numbers with Named Constants

Literal values scattered through code are a maintenance hazard. Extracting them into named constants centralizes meaning and makes their purpose searchable.

### Extract Interface

When a concrete type is used directly in multiple places, introducing an interface decouples callers from the specific implementation. This enables testing with fakes and future substitution of implementations.

## Structural Refactorings

### Move to Module

Logic placed in the wrong module creates surprising dependencies. Moving it to the module responsible for that concern restores cohesion and simplifies import graphs.

### Decompose Conditional

Complex boolean expressions buried in if-statements should be extracted into well-named predicates. `if is_eligible_for_discount(customer)` is clearer than the raw boolean arithmetic it replaces.

### Replace Nested Conditionals with Guard Clauses

Deeply nested if/else trees are replaced by early returns that handle special cases first, leaving the happy path as the final, unindented statement of the function.

## Refactoring Safety

No refactoring should be attempted without a comprehensive test suite. Automated formatting and linting prevent style debates during code review. Committing each small refactoring separately keeps history readable and bisectable.

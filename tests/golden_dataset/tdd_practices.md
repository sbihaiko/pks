# Test-Driven Development Practices

## Overview

Test-Driven Development (TDD) is a software development discipline in which tests are written before the production code they exercise. The short feedback loop forces clarity about desired behavior before implementation begins and produces a regression safety net as a natural by-product.

## The Red-Green-Refactor Cycle

TDD operates in three repeating phases. In the red phase, a failing test is written for the smallest piece of new behavior. The test must fail for the right reason — a compilation error or assertion failure, not an unexpected exception. In the green phase, the minimum production code needed to pass the test is written. No gold-plating, no premature abstraction. In the refactor phase, both the test and production code are cleaned up while keeping all tests green.

### Writing Good Unit Tests

A good unit test is fast, isolated, repeatable, self-validating, and timely (FIRST). It tests one behavior at a time and names that behavior clearly. The test name should read like a sentence describing a requirement, such as `calculates_total_price_including_tax`.

### Test Doubles

TDD relies on isolating the unit under test. Stubs return canned values; fakes have working but simplified implementations; mocks verify interaction patterns. Overusing mocks couples tests to implementation rather than behavior, making refactoring painful.

## Acceptance Test-Driven Development

At the feature level, ATDD extends TDD outward. Acceptance tests, written in collaboration with stakeholders before development begins, define the feature boundary. Unit tests fill in the internal structure. This two-level loop ensures that passing unit tests actually deliver user value.

## Common Pitfalls

Writing tests after code defeats the design feedback. Testing implementation details instead of behavior makes tests brittle. Ignoring failing tests erodes trust in the suite. Skipping the refactor phase accumulates design debt faster than coding without tests at all.

## Measuring Test Quality

Mutation testing complements code coverage by injecting faults into production code and verifying that tests catch them. A suite that passes mutation testing is far more reliable than one that merely achieves high line coverage.

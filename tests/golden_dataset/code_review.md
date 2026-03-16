# Effective Code Review

## Overview

Code review is a quality gate and knowledge-sharing mechanism. It catches defects before they reach production, spreads architectural understanding across the team, and maintains consistency in style and design patterns. Effective code review requires clear norms, psychological safety, and disciplined scope.

## Review Goals

Code review serves multiple purposes simultaneously. Correctness checks verify that the code does what it claims. Design review asks whether the approach fits the architecture. Readability review ensures that future maintainers can understand the code without the author present. Security review looks for common vulnerabilities: injection, broken access control, insecure deserialization.

## Writing Reviewable Changes

Small changes are easier to review accurately than large ones. A change that addresses one concern, fits on a single screen, and includes a clear description of the problem and solution receives more useful feedback than a sprawling refactor. Separate refactoring commits from behavior changes to keep review focused.

### Describing the Change

A pull request description should explain why the change exists, what approach was chosen, and what alternatives were considered. Screenshots, benchmark results, and links to related issues reduce the reviewer's need to reconstruct context from code alone.

## Giving Feedback

Feedback should address the code, not the author. Distinguishing between blocking issues (must change before merge), suggestions (would improve the code), and nits (style preferences) helps the author triage feedback efficiently. Asking questions rather than making demands invites dialogue and surfaces assumptions.

## Responding to Feedback

Authors should treat review feedback as information, not criticism. Every comment deserves a response: either a code change or an explanation of why the current approach is correct. Unresolved threads should not block indefinitely; time-box discussions and escalate to synchronous conversation when async is not converging.

## Automation

Automated checks — linting, formatting, type checking, security scanning — remove mechanical feedback from human review. Reviewers can then focus on logic, design, and intent rather than style violations.

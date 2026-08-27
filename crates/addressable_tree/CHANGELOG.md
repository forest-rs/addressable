# Changelog

## 0.1.0 - 2026-08-27

Initial release of reusable `no_std + alloc` execution over host-owned rooted
trees and forests:

- exact, relative, and pinned locator resolution;
- deterministic tree queries with explicit cardinality and budgets;
- host-reported predicate work accounting;
- indexed movement detection through overridable referent occurrence lookup;
- revision-safe host commits, replacement, extraction, and reconstruction;
- borrowed-host support through `TreeHost for &H`.

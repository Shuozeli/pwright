# Bug Reports

Detailed investigation reports for significant bugs. For the prioritized
bug/improvement list, see [known-issues.md](../known-issues.md).

## Reports

| Report | Status | Severity | Summary |
|--------|--------|----------|---------|
| [tab-leak-bug-report](tab-leak-bug-report.md) | Fixed | Critical | `with_page` leaked tabs when CDP WebSocket was dead. Removed `with_page`; callers use explicit `new_tab`/`close` lifecycle. |

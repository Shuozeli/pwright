# Bug Reports

Detailed investigation reports for significant bugs. For the prioritized
bug/improvement list, see [known-issues.md](../known-issues.md).

## Reports

| Report | Status | Severity | Summary |
|--------|--------|----------|---------|
| [tab-leak-bug-report](tab-leak-bug-report.md) | Fixed | Critical | Tabs leaked when CDP WebSocket died under Chrome memory pressure. Removed `with_page`; added `TabCloser` trait with `HttpTabCloser` that uses Chrome HTTP endpoint for reliable close. |

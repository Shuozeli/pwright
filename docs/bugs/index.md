# Bug Reports

Detailed investigation reports for significant bugs. For the prioritized
bug/improvement list, see [known-issues.md](../known-issues.md).

## Reports

| Report | Status | Severity | Summary |
|--------|--------|----------|---------|
| [targetinfo-http-parse-failure](targetinfo-http-parse-failure.md) | Fixed | High | `TargetInfo` struct uses CDP field names (`targetId`, required `attached`) but `ChromeHttpClient::list_targets()` parses Chrome HTTP `/json/list` which uses `id` and omits `attached`. Fixed: added `#[serde(alias = "id")]` and `#[serde(default)]`. Also fixed `create_target` to use PUT for Chrome 134+. |
| [tab-leak-bug-report](tab-leak-bug-report.md) | Fixed | Critical | Tabs leaked when CDP WebSocket died under Chrome memory pressure. Removed `with_page`; added `TabCloser` trait with `HttpTabCloser` that uses Chrome HTTP endpoint for reliable close. |
| [gmail-cdp-click-limitation](gmail-cdp-click-limitation.md) | Fixed | High | Gmail and heavy SPAs return empty accessibility snapshots and ignore JS-dispatched events. Fixed: added `click-at`, `hover-at`, `dblclick` CLI commands for coordinate-based real CDP input. |

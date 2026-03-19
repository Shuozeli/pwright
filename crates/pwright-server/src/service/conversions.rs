//! Type conversions between bridge/CDP types and proto types.

use pwright_bridge::snapshot::A11yNode;
use pwright_cdp::domains::network::Cookie;
use pwright_cdp::domains::target::TargetInfo;

use crate::proto;

impl From<A11yNode> for proto::A11yNode {
    fn from(n: A11yNode) -> Self {
        Self {
            r#ref: n.ref_id,
            role: n.role,
            name: n.name,
            depth: n.depth,
            value: n.value,
            disabled: n.disabled,
            focused: n.focused,
            node_id: n.node_id,
        }
    }
}

impl From<Cookie> for proto::CookieEntry {
    fn from(c: Cookie) -> Self {
        Self {
            name: c.name,
            value: c.value,
            domain: c.domain,
            path: c.path,
            expires: c.expires,
            http_only: c.http_only,
            secure: c.secure,
            same_site: c.same_site,
        }
    }
}

impl From<proto::CookieEntry> for Cookie {
    fn from(c: proto::CookieEntry) -> Self {
        Self {
            name: c.name,
            value: c.value,
            domain: c.domain,
            path: c.path,
            expires: c.expires,
            http_only: c.http_only,
            secure: c.secure,
            same_site: c.same_site,
        }
    }
}

impl From<TargetInfo> for proto::TabInfo {
    fn from(t: TargetInfo) -> Self {
        Self {
            target_id: t.target_id,
            r#type: t.target_type,
            title: t.title,
            url: t.url,
        }
    }
}

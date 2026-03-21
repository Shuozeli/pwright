# Zhihu HTML Structure

Explored 2026-03-21 against zhihu.com.

## Hot Page (`/hot`) — Requires Login

Redirects to `/signin` if not logged in.

### Key Element: `.HotItem`

Each item in the hot list is a `div.HotItem`.

| Selector | Content |
|----------|---------|
| `.HotItem-title` | Question title (Chinese) |
| `.HotItem-excerpt` | Answer excerpt |
| `.HotItem-metrics` | Heat metric (e.g., "606 万热度") |
| `.HotItem-title` closest `a` | Question URL |
| `.HotItem-index` | Rank number |

### Extraction JS

```js
Array.from(document.querySelectorAll(".HotItem")).map(item => ({
    title: item.querySelector(".HotItem-title")?.textContent?.trim(),
    excerpt: item.querySelector(".HotItem-excerpt")?.textContent?.trim(),
    heat: item.querySelector(".HotItem-metrics")?.textContent?.trim()?.replace("​分享", ""),
    url: item.querySelector("a")?.href,
}))
```

## Explore Page (`/explore`) — No Login Required

Has special topic cards, roundtable cards, and collection cards.
Less structured than hot page — better for topic discovery than feed extraction.

### Key Elements

| Class | Content |
|-------|---------|
| `ExploreSpecialCard` | Curated topic collections |
| `ExploreRoundtableCard` | Discussion roundtables |
| `ExploreCollectionCard` | Question collections |

## Login Detection

```js
// Hot page redirects to /signin if not logged in
window.location.href.includes("/signin") ? "logged_out" : "logged_in"
```

## Notes

- Hot page (`/hot`) is the most valuable — ranked by heat metric
- Explore page (`/explore`) works without login but has less structured data
- All content is Chinese
- Questions link to `/question/{id}` URLs
- Heat metric format: "N 万热度" (N x 10,000 views)

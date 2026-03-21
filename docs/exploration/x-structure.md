# X.com (Twitter) HTML Structure

Explored 2026-03-20 against x.com (logged in).

## Key Element: `[data-testid="tweet"]`

Each tweet is a `<div>` (actually `<article>`) with `data-testid="tweet"`.
Data is in child elements, not attributes.

## Internal selectors

| data-testid | Content |
|-------------|---------|
| `Tweet-User-Avatar` | Avatar container |
| `User-Name` | Multi-line text: `DisplayName\n@handle\n.\nTime` |
| `tweetText` | Tweet body text |
| `tweetPhoto` | Image (if present) |
| `card.wrapper` | Link card (if present) |
| `like` | `aria-label` has "N Likes. Like" |
| `retweet` | `aria-label` has "N reposts. Repost" |
| `reply` | `aria-label` has "N Replies. Reply" |
| `bookmark` | Bookmark button |
| `icon-verified` | Blue check (if present) |

## Status URL

Each tweet has an `<a>` with `href` containing `/status/ID`.
Select via: `t.querySelector("a[href*='/status/']")?.href`

## Engagement parsing

The `aria-label` on like/retweet/reply contains "N Action. Action":
```
"2030 Likes. Like"  -> 2030
"543 reposts. Repost" -> 543
```

Parse with: `parseInt(el.getAttribute("aria-label"))`

## User-Name parsing

`User-Name` text splits on `\n`:
```
parts[0] = "CNN"       // display name
parts[1] = "@CNN"      // handle
parts[3] = "Mar 20"    // relative time
```

## Extraction JS

```js
Array.from(document.querySelectorAll("[data-testid='tweet']")).map(t => {
    const parts = t.querySelector("[data-testid='User-Name']")?.innerText?.split("\n") || [];
    return {
        displayName: parts[0],
        handle: parts[1],
        time: parts[3],
        text: t.querySelector("[data-testid='tweetText']")?.innerText,
        likes: parseInt(t.querySelector("[data-testid='like']")?.getAttribute("aria-label")) || 0,
        retweets: parseInt(t.querySelector("[data-testid='retweet']")?.getAttribute("aria-label")) || 0,
        replies: parseInt(t.querySelector("[data-testid='reply']")?.getAttribute("aria-label")) || 0,
        link: t.querySelector("a[href*='/status/']")?.href,
        verified: !!t.querySelector("[data-testid='icon-verified']"),
    };
})
```

## Notes

- Requires login (timeline is personalized)
- Feed is infinite-scroll
- Tweet structure is stable across home, search, and profile pages
- `data-testid` attributes appear to be stable (React testing IDs)

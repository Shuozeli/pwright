# Weibo HTML Structure

Explored 2026-03-21 against weibo.com (logged in).

## Hot Search (`/hot/search`)

Uses Vue virtual scroller (`vue-recycle-scroller`).

### Key Element: `.wbpro-scroller-item`

Each hot topic is inside a virtual scroller item.

Text content splits on `\n`:
- parts[0]: rank number (or title for pinned item)
- parts[1]: topic title
- parts[2]: tag ("hot", "boiling", etc.) or heat number
- parts[3]: heat number (if tag present)

Links go to `s.weibo.com/weibo?q=...` search URLs.

### Extraction JS

```js
Array.from(document.querySelectorAll('.wbpro-scroller-item')).map(item => {
    const text = item.innerText?.split('\n').filter(Boolean);
    const link = item.querySelector('a')?.href;
    // First item is pinned (no rank number)
    const isRanked = /^\d+$/.test(text[0]);
    return {
        rank: isRanked ? parseInt(text[0]) : 0,
        title: isRanked ? text[1] : text[0],
        tag: isRanked ? text[2] : text[1],
        heat: parseInt(isRanked ? text[3] : text[2]) || 0,
        url: link,
    };
})
```

## Home Feed (`/`)

### Key Element: `.wbpro-scroller-item` containing `.wbpro-feed-content`

| Selector | Content |
|----------|---------|
| `.wbpro-feed-ogText` | Original post text |
| `.wbpro-feed-reText` | Repost text (if repost) |
| `[class*=head_info] a[class*=name]` | Author name |
| `[class*=head-info_time]` | Timestamp |
| `a[href*="/detail/"]` | Post detail link |

### Feed Extraction JS

```js
Array.from(document.querySelectorAll('.wbpro-scroller-item')).map(item => {
    const nameEl = item.querySelector('[class*=head_info] a[class*=name]')
        || item.querySelector('[class*=head-info] a');
    return {
        author: nameEl?.textContent?.trim(),
        text: item.querySelector('.wbpro-feed-ogText')?.innerText?.trim()?.slice(0, 300),
        repost: item.querySelector('.wbpro-feed-reText')?.innerText?.trim()?.slice(0, 200) || null,
        time: item.querySelector('[class*=head-info_time]')?.textContent?.trim(),
        link: item.querySelector('a[href*="/detail/"]')?.href,
    };
})
```

## Notes

- Requires login for both feed and hot search
- Uses Vue virtual scroller — only visible items are in DOM
- CSS class names are obfuscated with hashes (e.g., `_wrap_s5b56_2`)
- Use `wbpro-*` class prefixes which are stable
- Hot search heat numbers are view counts

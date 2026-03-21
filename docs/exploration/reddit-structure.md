# Reddit HTML Structure

Explored 2026-03-20 against reddit.com (new Reddit UI).

## Key Element: `<shreddit-post>`

Reddit uses Web Components. Each post is a `<shreddit-post>` custom element with
all data exposed as HTML attributes:

| Attribute | Type | Example |
|-----------|------|---------|
| `post-title` | string | "What is the most undervalued 10x play..." |
| `permalink` | path | "/r/stocks/comments/1rwqh9n/..." |
| `score` | number string | "518" |
| `author` | string | "Comfortable-Rule-491" |
| `comment-count` | number string | "1013" |
| `subreddit-prefixed-name` | string | "r/stocks" |
| `created-timestamp` | ISO 8601 | "2026-03-18T01:42:17.184000+0000" |
| `flair-text-content` | string | null if no flair |

## Feed container

Posts are inside `<shreddit-feed>`. The feed is infinite-scroll.

## Extraction JS

```js
Array.from(document.querySelectorAll("shreddit-post")).map(p => ({
    title: p.getAttribute("post-title"),
    permalink: p.getAttribute("permalink"),
    score: p.getAttribute("score"),
    author: p.getAttribute("author"),
    commentCount: p.getAttribute("comment-count"),
    subreddit: p.getAttribute("subreddit-prefixed-name"),
    created: p.getAttribute("created-timestamp"),
    flair: p.getAttribute("flair-text-content"),
}))
```

## Notes

- Works on front page (feed) and subreddit pages (e.g., `/r/rust`)
- Subreddit pages show the same `shreddit-post` structure
- No login required for public subreddits
- `permalink` is a relative path, prefix with `https://www.reddit.com`

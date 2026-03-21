# GitHub Trending HTML Structure

Explored 2026-03-21 against github.com/trending.

## Key Element: `article.Box-row`

Each trending repo is an `<article>` with class `Box-row`.

| Selector | Content |
|----------|---------|
| `h2 a` | Repo link (`href="/owner/repo"`) |
| `p` | Description text |
| `[itemprop="programmingLanguage"]` | Primary language |
| `a.Link--muted` (1st) | Star count (e.g., "17,378") |
| `a.Link--muted` (2nd) | Fork count (e.g., "1,849") |
| `.float-sm-right` | Today's stars (e.g., "379 stars today") |

## Extraction JS

```js
Array.from(document.querySelectorAll("article.Box-row")).map(r => {
    const links = Array.from(r.querySelectorAll("a.Link--muted"));
    return {
        repo: r.querySelector("h2 a")?.getAttribute("href")?.slice(1),
        description: r.querySelector("p")?.textContent?.trim(),
        language: r.querySelector("[itemprop='programmingLanguage']")?.textContent?.trim() || null,
        stars: parseInt(links[0]?.textContent?.trim()?.replace(/,/g, "")) || 0,
        forks: parseInt(links[1]?.textContent?.trim()?.replace(/,/g, "")) || 0,
        todayStars: parseInt(r.querySelector(".float-sm-right")?.textContent) || 0,
    };
})
```

## URL Parameters

- `/trending` — all languages, daily
- `/trending?since=weekly` — weekly
- `/trending?since=monthly` — monthly
- `/trending/rust` — specific language
- `/trending/rust?since=weekly` — language + period

## Notes

- No login required
- Stable selectors (GitHub rarely changes trending page)
- `itemprop="programmingLanguage"` is semantic HTML, very stable
- Star/fork counts have commas, parse with `replace(/,/g, "")`

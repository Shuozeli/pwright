# Wikipedia HTML Structure

Explored 2026-03-21 against en.wikipedia.org.

## Article Page

### Key Elements

| Selector | Content |
|----------|---------|
| `#firstHeading` | Article title |
| `.mw-parser-output > p:not(.mw-empty-elt)` | First paragraph (summary) |
| `.mw-heading2 h2` | Section headings (H2 level) |
| `.mw-heading3 h3` | Subsection headings (H3 level) |
| `.infobox` | Infobox sidebar table (if present) |
| `#mw-normal-catlinks li a` | Categories |
| `#toc` or `.toc` | Table of contents |

### Infobox Extraction

```js
(() => {
    const ib = document.querySelector(".infobox");
    if (!ib) return null;
    const rows = {};
    ib.querySelectorAll("tr").forEach(r => {
        const th = r.querySelector("th");
        const td = r.querySelector("td");
        if (th && td) rows[th.textContent.trim()] = td.textContent.trim();
    });
    return rows;
})()
```

Note: Infobox `td` content may include CSS from inline styles (`.mw-parser-output` rules
leak into `textContent`). Use `innerText` instead of `textContent` for cleaner output.

### Full Extraction JS

```js
(() => {
    const firstP = document.querySelector(".mw-parser-output > p:not(.mw-empty-elt)");
    return {
        title: document.querySelector("#firstHeading")?.innerText?.trim(),
        summary: firstP?.innerText?.trim(),
        sections: Array.from(document.querySelectorAll(".mw-heading2 h2"))
            .map(h => h.innerText?.replace("[edit]", "").trim()),
        categories: Array.from(document.querySelectorAll("#mw-normal-catlinks li a"))
            .map(a => a.textContent),
        url: window.location.href,
    };
})()
```

## URL Format

- `https://en.wikipedia.org/wiki/{Article_Title}` — English
- `https://zh.wikipedia.org/wiki/{Article_Title}` — Chinese
- Spaces in titles are underscores: `Rust_(programming_language)`

## Notes

- No login required
- Extremely stable DOM (MediaWiki hasn't changed major selectors in years)
- Use `innerText` over `textContent` to avoid CSS rule leakage from infobox
- `[edit]` suffix appears in section headings — strip it
- Some articles have no infobox

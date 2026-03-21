# Xueqiu HTML Structure

Explored 2026-03-21 against xueqiu.com (logged in).

## Stock Quote Page (`/S/{SYMBOL}`)

### Key Elements

| Selector | Content | Example |
|----------|---------|---------|
| `.stock-name` | Stock name + exchange | "Apple(NASDAQ:AAPL)" |
| `.stock-current` | Current price | "$247.99" |
| `.stock-change` | Change + percentage | "-0.97 -0.39%" |
| `.stock-info td` | Key stats table | Label in `<span>`, value as text |

### Symbols

- US stocks: `AAPL`, `TSLA`, `AMZN`
- CN A-shares: `SH600519` (Kweichow Moutai), `SZ000001` (Ping An)
- HK stocks: `00700` (Tencent), `09988` (Alibaba)
- Index: `SH000001` (SSE Composite), `.DJI` (Dow Jones)

### Quote Extraction JS

```js
(() => {
    const stats = {};
    document.querySelectorAll('.stock-info td').forEach(td => {
        const label = td.querySelector('span')?.textContent?.trim();
        const value = td.textContent?.replace(label || '', '').trim();
        if (label) stats[label] = value;
    });
    return {
        name: document.querySelector('.stock-name')?.innerText?.trim(),
        price: document.querySelector('.stock-current')?.innerText?.trim(),
        change: document.querySelector('.stock-change')?.innerText?.trim(),
        stats,
        url: window.location.href,
    };
})()
```

### Key Stats Fields (Chinese labels)

| Label | Meaning |
|-------|---------|
| 最高 | Day high |
| 最低 | Day low |
| 今开 | Open |
| 昨收 | Previous close |
| 成交量 | Volume |
| 成交额 | Turnover |
| 总市值 | Market cap |
| 市盈率(TTM) | P/E ratio (TTM) |
| 市净率 | P/B ratio |
| 52周最高 | 52-week high |
| 52周最低 | 52-week low |
| 股息率(TTM) | Dividend yield |
| 每股收益 | EPS |

## Notes

- Login recommended (some features limited without it)
- Stock pages are public but watchlist/portfolio require login
- URL format: `xueqiu.com/S/{SYMBOL}`
- Price shown in original currency (USD for US, CNY for A-shares, HKD for HK)

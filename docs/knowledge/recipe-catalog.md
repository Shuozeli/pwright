# Recipe Catalog

Proposed full recipe list. Recipes marked [EXISTS] are already implemented.
Recipes marked [PROPOSED] need exploration + implementation.

## Directory Structure

```
examples/recipes/
  research/
    news/           News aggregators and feeds
    social/         Social media platforms
    forums/         Discussion forums and Q&A
    finance/        Financial data and markets
    search/         Search engines
    knowledge/      Reference and encyclopedia sites
    generic/        Site-agnostic extractors
  communication/    Email, messaging, notifications
  monitoring/       Health checks, dashboards
  automation/       Forms, logins, workflows
```

## Research: News

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| hackernews-feed | news.ycombinator.com | No | [EXISTS] | `.athing` rows + `.subtext` |
| techmeme-feed | techmeme.com | No | [PROPOSED] | Top tech news aggregator |
| producthunt-feed | producthunt.com | No | [PROPOSED] | Daily product launches |

## Research: Social

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| x-feed | x.com | Yes | [EXISTS] | `data-testid="tweet"` timeline |
| x-search | x.com | Yes | [EXISTS] | Search with tab selection |
| x-profile | x.com | Yes | [PROPOSED] | Extract profile + pinned tweets |
| reddit-feed | reddit.com | No | [EXISTS] | `shreddit-post` web components |
| reddit-search | reddit.com | No | [EXISTS] | Search with sort options |
| reddit-post | reddit.com | No | [PROPOSED] | Single post + top comments |
| weibo-hot | weibo.com | Yes | [EXISTS] | Hot search trending topics |
| weibo-feed | weibo.com | Yes | [EXISTS] | Home timeline posts |
| weibo-search | weibo.com | Yes | [PROPOSED] | Topic search results |
| linkedin-feed | linkedin.com | Yes | [PROPOSED] | Home feed posts |

## Research: Forums & Q&A

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| zhihu-hot | zhihu.com | Yes | [EXISTS] | Hot questions ranked by heat |
| zhihu-feed | zhihu.com | Yes | [EXISTS] | Recommendation feed with excerpts |
| zhihu-search | zhihu.com | No | [PROPOSED] | Search answers |
| zhihu-question | zhihu.com | No | [PROPOSED] | Single question + top answers |
| stackoverflow-question | stackoverflow.com | No | [PROPOSED] | Question + accepted answer + vote counts |
| v2ex-feed | v2ex.com | No | [PROPOSED] | Hot topics feed |

## Research: Finance

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| yahoo-finance-quote | finance.yahoo.com | No | [PROPOSED] | Stock quote + key stats |
| yahoo-finance-news | finance.yahoo.com | No | [PROPOSED] | Stock-related news |
| xueqiu-quote | xueqiu.com | No | [EXISTS] | Stock quote + 28 stats (US/CN/HK) |
| xueqiu-feed | xueqiu.com | No | [PROPOSED] | Hot stocks / timeline |
| eastmoney-news | eastmoney.com | No | [PROPOSED] | Financial news feed |
| google-finance-quote | google.com/finance | No | [PROPOSED] | Stock quote card |

## Research: Search

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| google-search | google.com | No | [EXISTS] | Organic results extraction |

## Research: Knowledge

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| wikipedia-article | wikipedia.org | No | [PROPOSED] | Article summary + sections |
| baike-article | baike.baidu.com | No | [PROPOSED] | Chinese encyclopedia article |
| github-trending | github.com | No | [PROPOSED] | Trending repositories list |
| github-repo | github.com | No | [PROPOSED] | Repo stars, forks, description, languages, recent activity |

## Research: Generic

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| extract-article | any | No | [EXISTS] | Readability-style article extraction |
| extract-table | any | No | [EXISTS] | HTML table to JSON |
| extract-links | any | No | [PROPOSED] | All links with context |
| extract-metadata | any | No | [PROPOSED] | Open Graph, title, description |

## Communication

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| gmail-read | mail.google.com | Yes | [EXISTS] | Read inbox emails |
| gmail-send | mail.google.com | Yes | [EXISTS] | Compose and send |
| github-notifications | github.com | Yes | [EXISTS] | Unread notifications |

## Monitoring

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| screenshot-dashboard | any | Varies | [EXISTS] | Screenshot any URL |
| check-deploy | any | No | [EXISTS] | Health check with status code |

## Automation

| Recipe | Site | Auth | Status | Notes |
|--------|------|:----:|--------|-------|
| login-and-extract | any | Yes | [EXISTS] | Login flow + data extraction |
| fill-and-submit | any | Varies | [EXISTS] | Form fill and submit |

## API Alternatives

These sites have APIs that may be better for **programmatic pipelines**.
Recipes are still useful for **ad-hoc agent use** (no API key setup needed,
works with any logged-in browser session):

| Site | API option | When to use recipe instead |
|------|------------|---------------------------|
| GitHub | `gh api` or REST API | No `gh` CLI auth configured; agent needs quick one-off lookup |
| StackOverflow | Stack Exchange API v2 | Reading a specific question page with full context |
| Baidu search | Anti-bot makes recipes unreliable | Avoid — use API or other search engines |

---

## Implementation Priority

**P0 (high value, stable selectors):**
- zhihu-feed, zhihu-search (large Chinese developer/knowledge community)
- weibo-feed, weibo-search (Chinese social media)
- github-trending (stable DOM, no auth, developer audience)
- wikipedia-article (extremely stable DOM)
- stackoverflow-question (single page content extraction, stable)

**P1 (useful, moderate stability):**
- yahoo-finance-quote (financial data, complex DOM)
- xueqiu-feed, xueqiu-quote (Chinese finance)
- reddit-post (single post + comments)
- x-profile (user profile data)
- baike-article (Chinese encyclopedia)

**P2 (nice to have):**
- techmeme-feed, producthunt-feed
- v2ex-feed, eastmoney-news
- linkedin-feed

## Recipe Freshness

Every recipe YAML header includes a `# Tested:` date and UI description.
Sites change frequently. When a recipe breaks:

1. Re-explore the site (dump HTML, find new selectors)
2. Update the extraction JS
3. Update the `# Tested:` date
4. Run the recipe to verify

Exploration docs in `docs/exploration/` document the current DOM structure
with examples, so future updates don't start from scratch.

# Script Examples

Sample YAML scripts for `pwright run`.

## Scripts

| Script | Description | Key features |
|--------|-------------|--------------|
| [hello.yaml](hello.yaml) | Navigate and extract title | Minimal example, params |
| [extract-links.yaml](extract-links.yaml) | Extract all links from a page | JS registry, eval with args |
| [login-and-scrape.yaml](login-and-scrape.yaml) | Log in then scrape | Multi-step, param-file for secrets |
| [form-fill.yaml](form-fill.yaml) | Fill and submit a form | fill, click, verify result |
| [screenshot-audit.yaml](screenshot-audit.yaml) | Audit page structure | extract visibility, multiple checks |

## Running

```bash
# Basic
pwright run examples/scripts/hello.yaml --param url=https://example.com

# With secrets file
pwright run examples/scripts/login-and-scrape.yaml \
  --param login_url=https://example.com/login \
  --param target_url=https://example.com/dashboard \
  --param-file secrets.yaml

# Validate without executing
pwright run examples/scripts/hello.yaml --param url=https://example.com --validate
```

## Output

Scripts produce JSONL (one JSON line per step):

```jsonl
{"step_index":0,"step_type":"goto","status":"ok","duration_ms":1200,"details":{"url":"https://example.com"}}
{"step_index":1,"step_type":"extract","status":"ok","duration_ms":5,"details":{"selector":"h1","field":"text_content","value":"Example Domain"}}
{"summary":true,"name":"Hello pwright","status":"ok","total_steps":3,"succeeded":3,"failed":0,"outputs":[{"heading":"Example Domain"}]}
```

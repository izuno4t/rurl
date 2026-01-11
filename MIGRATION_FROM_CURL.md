# Migration Guide: curl -> rurl

## Basics

- Most common curl options work with the same names.
- TLS validation and redirect behavior follow curl (`--location`, `--location-trusted`, etc.).

## Option Mapping

| curl | rurl | Notes |
| --- | --- | --- |
| `-X/--request` | same | |
| `-H/--header` | same | |
| `-d/--data` | same | POST/PUT switching follows curl |
| `-u/--user` | same | Basic/Bearer supported |
| `-L/--location` | same | Auth headers kept on same host |
| `--location-trusted` | same | Forward auth even to other hosts |
| `--max-redirs` | same | |
| `--retry`/`--retry-delay` | same | |
| `-k/--insecure` | same | Disables TLS verification (not recommended) |
| `-o/--output` | same | |
| `-i/--include` | same | Show response headers |

## rurl-specific features

- `--cookies-from-browser BROWSER[+KEYRING][:PROFILE][::CONTAINER]`
  - Chrome/Chromium/Edge/Brave/Opera/Vivaldi/Whale, Firefox, Safari (macOS) cookies directly
  - Automates what curl would require manual export for

## Migration tips

- Start by running your existing curl command with `rurl` and compare outputs.
- If auth headers drop on redirects, add `--location-trusted`.
- Prefer `--cacert` for cert issues; use `-k` only as a last resort.
- For charset issues, rely on automatic detection/`--json`, and specify `Accept-Charset` if needed.

## Packaging and publishing

- Binary distribution: `make dist`
- crates.io publishing: see `docs/PUBLISHING.md`

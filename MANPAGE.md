# rurl — transfer a URL with browser cookie support

## Synopsis

```text
rurl [options] [URL...]
```

## Description

`rurl` is a curl-compatible CLI built in Rust. It focuses on safe defaults, browser cookie integration,
and clear diagnostics. Most common curl flags are supported, and additional options help reuse browser
sessions across Chrome/Firefox/Chromium-based browsers.

## Common Options

- `-X, --request <METHOD>`: HTTP method (GET/POST/PUT/DELETE, etc.)
- `-H, --header <HEADER>`: Add request header
- `-d, --data <DATA>`: Send body (implies POST unless `--request` overrides)
- `-o, --output <FILE>`: Write response to file
- `-u, --user <USER[:PASSWORD]>`: HTTP basic auth
- `-L, --location`: Follow redirects (keep auth on same host; use `--location-trusted` to force)
- `--max-redirs <N>`: Limit redirects
- `--timeout <SECS>` / `--connect-timeout <SECS>`: Timeouts
- `--retry <N>` / `--retry-delay <SECS>`: Retry failed requests
- `-v, --verbose`: Verbose transfer logging
- `-s, --silent`: Suppress progress and errors
- `--insecure` (`-k`): Disable TLS verification (not recommended)

## Browser Cookie Integration

- `--cookies-from-browser BROWSER[+KEYRING][:PROFILE][::CONTAINER]`
  - Chrome/Chromium/Edge/Brave/Opera/Vivaldi/Whale, Firefox, Safari (macOS)
  - `+KEYRING` for Linux keyring, `:PROFILE` for named profile, `::CONTAINER` for Firefox container
- Cookies are filtered by domain/path/secure attributes before sending.

## Output Controls

- `--include` (`-i`): Include response headers
- `--json`: Pretty-print JSON when applicable
- `--progress` / default: Show progress; `--silent` disables

## Files and Environment

- Config is driven by CLI options; no global config file is required.
- Uses system certificate store via rustls-native-certs where available.

## Exit Codes

- Follows curl-style exit codes (e.g., 6 for malformed URL, 22 for HTTP error, 28 for timeout).

## Examples

- GET with headers:

  ```bash
  rurl -H "Accept: application/json" https://example.com/api
  ```

- POST JSON:

  ```bash
  rurl -X POST -H "Content-Type: application/json" -d '{"key":"val"}' https://example.com/api
  ```

- Use browser cookies:

  ```bash
  rurl --cookies-from-browser chrome https://example.com/profile
  ```

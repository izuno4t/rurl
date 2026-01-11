# rurl Tutorial

## Basics

### GET

```bash
rurl https://example.com
```

### Headers and query

```bash
rurl -H "Accept: application/json" "https://httpbin.org/get?foo=bar"
```

### POST (JSON)

```bash
rurl -X POST -H "Content-Type: application/json" -d '{"k":"v"}' https://httpbin.org/post
```

## Authentication

### Basic auth

```bash
rurl -u user:pass https://example.com/private
```

### Browser cookies

```bash
rurl --cookies-from-browser chrome https://example.com/profile
rurl --cookies-from-browser firefox:Profile1 https://example.com/profile
```

## Redirects and retries

```bash
# Follow redirects
rurl -L https://example.com

# Retry failed requests
rurl --retry 3 --retry-delay 2 https://flaky.example.com
```

## Proxy and TLS

```bash
# HTTP proxy
rurl -x http://proxy.local:8080 https://example.com

# Custom CA
rurl --cacert /path/ca.pem https://example.com

# Skip verification (not recommended)
rurl -k https://example.com
```

## Output control

```bash
# Save response to file
rurl -o out.json https://example.com/data

# Include response headers
rurl -i https://example.com

# Pretty-print JSON
rurl --json https://httpbin.org/json
```

## Common browser syntax

- Chrome family: `--cookies-from-browser chrome[:Profile]`
- Firefox container: `--cookies-from-browser firefox:Profile::Container`
- Linux keyring: `--cookies-from-browser chrome+KEYRING`

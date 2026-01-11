# rurl

[![CI][ci-badge]][ci-link]
[![Basic Build Check][basic]][ci-link]

[ci-badge]: https://github.com/izuno4t/rurl/actions/workflows/ci.yml/badge.svg
[basic]: https://github.com/izuno4t/rurl/actions/workflows/basic.yml/badge.svg
[ci-link]: https://github.com/izuno4t/rurl/actions

A Modern curl Alternative Written in Rust

## What is rurl?

rurl is a ground-up reimplementation of the classic curl command-line tool,
built with Rust for the modern web ecosystem. While maintaining compatibility
with curl's most commonly used features, rurl extends functionality with native
browser integration, allowing you to leverage your existing browser sessions for
authenticated API requests and web scraping.

### Why rurl over curl?

**🔒 Memory Safety**: Written in Rust, rurl eliminates entire classes of
security vulnerabilities common in C programs, such as buffer overflows and
use-after-free errors.

**🍪 Browser Integration**: Unlike curl, which requires manual cookie
management, rurl can directly access cookies from your installed browsers
(Chrome, Firefox, Safari, Edge, Brave), making authenticated requests trivial.

**⚡ Performance**: Leverages Rust's zero-cost abstractions and modern
async/await patterns for optimal performance, especially for concurrent
requests.

**🎯 Modern Defaults**: Sensible defaults for the modern web - automatic
decompression, UTF-8 handling, and JSON pretty-printing out of the box.

**🔧 Better Error Messages**: Clear, actionable error messages that tell you
exactly what went wrong and how to fix it.

### Philosophy

rurl is designed around three core principles:

1. **Compatibility First**: Common curl commands should "just work" in rurl
2. **Safety by Default**: Memory safety and secure defaults without sacrificing
   performance
3. **Browser-Native**: Treat browser cookies as first-class citizens, not an
   afterthought

### Use Cases

- **Authenticated API Testing**: Test protected endpoints using your browser's
  auth session
- **Web Scraping**: Scrape authenticated content without managing complex login
  flows
- **Development Workflows**: Seamlessly integrate with web services you're
  already logged into
- **Security Research**: Safely examine requests and responses with memory-safe
  tooling
- **CI/CD Pipelines**: Drop-in curl replacement with enhanced safety
  guarantees

## Installation & Building

### Prerequisites

- **Rust**: 1.92 or later (verified in CI; latest stable is recommended for
  local builds)
- **Git**: For cloning the repository

### Building from Source

```bash
# Clone the repository
git clone https://github.com/izuno4t/rurl.git
cd rurl

# Build in development mode
cargo build

# Build optimized release version
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

### Development

```bash
# Check code formatting
cargo fmt --all -- --check

# Run linter
cargo clippy --all-targets -- -D warnings

# Check compilation without building
cargo check
```

### Setup

Run the setup script to install local tooling (uses `rust-toolchain.toml`
channel):

```bash
./setup.sh
```

### Coverage

Local coverage requires `cargo-llvm-cov`:

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
make coverage
```

`make verify` also runs coverage via `coverage-ci`.

**Local environment constraints**:
If you are using an older Rust version (for example, 1.67.0), you will not be
able to build locally. GitHub Actions is configured to automatically verify
code quality and builds.

### **Man Page Synopsis**

```text
NAME
       rurl - transfer a URL with browser cookie support

SYNOPSIS
       rurl [options] [URL...]

DESCRIPTION
       rurl is a tool to transfer data from or to a server, using one of the
       supported protocols (HTTP, HTTPS, FTP, FTPS, SMTP, and more). The
       command is designed to work without user interaction and provides
       seamless integration with browser cookie stores.

       rurl offers a busload of useful tricks like proxy support, user
       authentication, FTP upload, HTTP post, SSL connections, browser
       cookies, file transfer resume, and more. As you will see below, the
       number of features will make your head spin!

       rurl is powered by Rust and libcurl-rust, offering memory safety
       guarantees while maintaining compatibility with curl's command-line
       interface.

OPTIONS
       Browser Cookie Integration:
       --cookies-from-browser BROWSER[+KEYRING][:PROFILE][::CONTAINER]
              Extract cookies directly from installed browsers. Supported
              browsers include: brave, chrome, chromium, edge, firefox, opera,
              safari, vivaldi, whale.

              Examples:
              firefox               - Use default Firefox profile
              firefox:Profile1      - Use named Firefox profile
              chrome                - Use default Chrome profile
              safari                - Use Safari cookies
              edge                  - Use Microsoft Edge cookies

              Format details:
              BROWSER    - Browser name (required)
              +KEYRING   - Keyring for decrypting Chromium cookies on Linux
                          (optional)
              :PROFILE   - Specific browser profile name (optional)
              ::CONTAINER - Firefox container name (optional)

       --cookies-from-browser-profile PROFILE
              Specify browser profile when using --cookies-from-browser

       Standard curl Options (Inherited):
       -X, --request METHOD
              HTTP request method (GET, POST, PUT, DELETE, etc.)

       -H, --header "HEADER: VALUE"
              Add custom HTTP headers

       -d, --data DATA
              HTTP POST data

       -o, --output FILE
              Write output to file instead of stdout

       -u, --user USER[:PASSWORD]
              HTTP authentication credentials

       -x, --proxy [PROTOCOL://]HOST[:PORT]
              Use proxy server

       --proxy-user USER[:PASSWORD]
              Proxy authentication credentials

       --cacert FILE
              CA certificate bundle file

       --cert FILE
              Client certificate file

       --key FILE
              Private key file for client certificate

       --insecure
              Allow insecure SSL connections

       -k, --insecure
              Alias for --insecure

       -v, --verbose
              Verbose output for debugging

       -s, --silent
              Silent mode - no progress or error output

       --user-agent STRING
              Set User-Agent header

       -L, --location
              Follow HTTP redirects

       --max-redirs NUMBER
              Maximum number of redirects to follow

       --timeout SECONDS
              Maximum time for operation

       --connect-timeout SECONDS
              Maximum time for connection

       --retry NUMBER
              Number of retry attempts

       --retry-delay SECONDS
              Delay between retries

EXAMPLES
       Basic HTTP request:
       rurl https://api.example.com/data

       Use browser cookies from Chrome:
       rurl --cookies-from-browser chrome https://authenticated-site.com/api

       Use specific Firefox profile:
       rurl --cookies-from-browser firefox:work https://work-internal-api.com

       Use Firefox container:
       rurl --cookies-from-browser firefox::Personal https://site.com

       POST request with browser authentication:
       rurl --cookies-from-browser chrome -X POST -H \
       "Content-Type: application/json" -d '{"key":"value"}' \
       https://api.example.com/submit

       Use proxy with authentication:
       rurl --proxy http://proxy.company.com:8080 --proxy-user user:pass \
       https://external-api.com

       Custom CA certificate:
       rurl --cacert /path/to/ca-bundle.pem https://self-signed-site.com

SUPPORTED PLATFORMS
       rurl supports cookie extraction across multiple operating systems:

       - Linux: All supported browsers
       - macOS: All supported browsers including Safari
       - Windows: All supported browsers

SECURITY CONSIDERATIONS
       - Cookies contain sensitive authentication data
       - Only use trusted networks when extracting browser cookies
       - Consider using --output to save responses rather than displaying in
         terminal
       - Browser cookies may require elevated privileges on some systems

BROWSER-SPECIFIC NOTES
       Chrome/Chromium-based:
       - Requires browser to be closed on Windows for cookie extraction
       - Uses AES encryption for cookie storage
       - May require admin privileges on Windows for Chrome 130+

       Firefox:
       - Stores cookies in unencrypted SQLite database
       - Supports container tabs for isolation
       - Works while browser is running

       Safari (macOS only):
       - Uses binary cookie format
       - Requires access to ~/Library/Cookies/

       Edge:
       - Similar to Chrome (Chromium-based)
       - Windows integration for authentication

EXIT STATUS
       0      Success
       1      General error
       2      Misused shell command
       3      Invalid URL
       4      Authentication required
       5      Proxy error
       6      Could not resolve host
       7      Failed to connect
       22     HTTP error
       35     SSL/TLS error

SEE ALSO
       curl(1), wget(1), yt-dlp(1)

AUTHOR
       rurl development team

COPYRIGHT
       This is free software; see the source for copying conditions.
```

# curlからrurlへの移行ガイド

## 基本対応

- ほとんどの一般的なcurlオプションは同名で利用可能です。
- HTTPS検証やリダイレクトの挙動はcurlに合わせています（`--location`、`--location-trusted`など）。

## 主なオプション対応表

| curl | rurl | 備考 |
| --- | --- | --- |
| `-X/--request` | 同じ | |
| `-H/--header` | 同じ | |
| `-d/--data` | 同じ | POST/PUT切替はcurlに準拠 |
| `-u/--user` | 同じ | Basic/Bearer対応 |
| `-L/--location` | 同じ | 認証ヘッダーは同一ホストのみ維持 |
| `--location-trusted` | 同じ | 別ホストへも認証情報を転送 |
| `--max-redirs` | 同じ | |
| `--retry`/`--retry-delay` | 同じ | |
| `-k/--insecure` | 同じ | TLS検証を無効化（非推奨） |
| `-o/--output` | 同じ | |
| `-i/--include` | 同じ | ヘッダー含めて表示 |

## rurl特有の機能

- `--cookies-from-browser BROWSER[+KEYRING][:PROFILE][::CONTAINER]`
  - Chrome/Chromium/Edge/Brave/Opera/Vivaldi/Whale、Firefox、Safari(macOS)のCookieを直接使用
  - curlでは手動エクスポートが必要な部分を自動化

## 移行時のヒント

- まずは既存のcurlコマンドをそのままrurlに置き換えて動作確認してください。
- 認証付きリダイレクトで資格情報が落ちる場合は `--location-trusted` を追加。
- 証明書エラーはまず `--cacert` で解決し、`-k` は最後の手段にしてください。
- 文字化けがある場合は `--json` / 自動charset判定を確認し、必要に応じて `Accept-Charset` を指定。

## パッケージングと公開

- バイナリ配布: `make dist`
- crates.io 公開: `docs/PUBLISHING.md` を参照

# rurl チュートリアル

## 基本

### GET

```bash
rurl https://example.com
```

### ヘッダーとクエリ

```bash
rurl -H "Accept: application/json" "https://httpbin.org/get?foo=bar"
```

### POST（JSON）

```bash
rurl -X POST -H "Content-Type: application/json" -d '{"k":"v"}' https://httpbin.org/post
```

## 認証

### Basic認証

```bash
rurl -u user:pass https://example.com/private
```

### ブラウザCookie

```bash
rurl --cookies-from-browser chrome https://example.com/profile
rurl --cookies-from-browser firefox:Profile1 https://example.com/profile
```

## リダイレクトとリトライ

```bash
# リダイレクトを追跡
rurl -L https://example.com

# リトライを設定
rurl --retry 3 --retry-delay 2 https://flaky.example.com
```

## プロキシとTLS

```bash
# HTTPプロキシ
rurl -x http://proxy.local:8080 https://example.com

# カスタムCA
rurl --cacert /path/ca.pem https://example.com

# 検証をスキップ（推奨しません）
rurl -k https://example.com
```

## 出力制御

```bash
# レスポンスをファイルに保存
rurl -o out.json https://example.com/data

# ヘッダーも表示
rurl -i https://example.com

# JSON整形出力
rurl --json https://httpbin.org/json
```

## よく使うブラウザ構文

- Chrome系: `--cookies-from-browser chrome[:Profile]`
- Firefoxコンテナ: `--cookies-from-browser firefox:Profile::Container`
- Linuxキーリング: `--cookies-from-browser chrome+KEYRING`

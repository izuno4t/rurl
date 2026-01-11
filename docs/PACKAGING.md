# パッケージング手順

## 目的

各プラットフォーム向けのバイナリ配布物を作成します。サイズ削減を優先したリリースプロファイルを使用します。

## 前提

- `rust-toolchain.toml` 記載のツールチェーンをインストール済み
- 必要に応じてクロスターゲットを `rustup target add` で追加済み

## 手順

1. リリースバイナリをパッケージング

   ```bash
   make dist
   ```

   - 出力先: `dist/rurl-<version>-<host-target>.tar.gz`
   - 同梱物: バイナリ、`README.md`、`LICENSE`、`MANPAGE.md`、`TUTORIAL.md`、`MIGRATION_FROM_CURL.md`
2. クリーンアップ

   ```bash
   make dist-clean
   ```

## 補足

- リリースプロファイルは LTO・strip・`opt-level="z"`・`panic=abort` でサイズ重視です。
- スタックトレースが必要な場合は環境変数 `RUSTFLAGS="-C panic=unwind"` などで上書きしてください。

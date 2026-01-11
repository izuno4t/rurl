# crates.io公開手順

## 前提

- `cargo login` 済み（APIトークンを設定）
- `cargo publish --dry-run` が通ること
- `Cargo.toml` のバージョンを公開版へ更新済み

## 手順

1. 依存関係と成果物を検証

   ```bash
   make verify
   cargo package --list
   ```

2. パッケージ内容を確認（必要に応じて tar.gz を展開して中身を確認）

   ```bash
   cargo package
   ```

3. 公開

   ```bash
   cargo publish
   ```

4. タグ付け・リリースノート

   ```bash
   git tag -a v<version> -m "Release v<version>"
   git push origin v<version>
   ```

## 補足

- パッケージに含めるファイルは `Cargo.toml` の `include` で制御しています。
- リリースプロファイルはサイズ最適化（LTO/strip/`opt-level="z"`/`panic=abort`）。
- テスト・カバレッジは `make verify` でまとめて実行できます。

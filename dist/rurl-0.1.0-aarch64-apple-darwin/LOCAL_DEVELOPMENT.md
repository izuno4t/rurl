# ローカル開発環境について

## Rustバージョン要件

- **本プロジェクトのMSRV**: Rust 1.92以上
- **CI環境**: GitHub Actionsで1.92.0を使用
- **ローカル環境**: Rust 1.92.0以上を推奨（`rust-toolchain.toml`で固定）

## ローカル開発の制限事項

ローカルのRustが1.92未満の場合は、以下の制限があります：

### ビルド制限

- `cargo build`はローカルでは実行できません
- 依存関係が新しいRustバージョンを要求するため

### 推奨開発フロー

1. **コード編集**: IDEでコードを編集
2. **Rust更新**: `rustup toolchain install 1.92.0` を実行
3. **ツールチェーン設定**: `rustup default 1.92.0` で既定を設定
4. **CI確認**: GitHub Actionsでビルド・テストを確認
5. **機能実装**: 構造設計とロジック実装に集中
6. **最終確認**: CIパイプラインで品質保証

### ローカル一括チェック

複数のチェックをまとめて実行する場合は `Makefile` を使用します：

```bash
make verify
```

個別実行したい場合は以下を利用してください：

```bash
make fmt-check
make clippy
make check
make test
```

### セットアップスクリプト

開発に必要なツールは `setup.sh` でまとめて導入できます（`rust-toolchain.toml` の `channel` を使用します）：

```bash
./setup.sh
```

### テストカバレッジ計測（cargo-llvm-cov）

ローカルでカバレッジを取得するには、`cargo-llvm-cov` をインストールします。

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

カバレッジの取得は次のコマンドで実行できます：

```bash
make coverage
```

`make coverage` はHTMLレポートを生成してブラウザで開きます。
`make verify` は `coverage-ci` を含み、ブラウザを開かないカバレッジ計測を実行します。

### リリースビルド最適化

`cargo build --release` はバイナリサイズを優先した設定です（LTO、`codegen-units=1`、`opt-level=z`、
`strip=true`、`panic=abort`）。リリースビルドでスタックトレースを取りたい場合は
`RUSTFLAGS="-C panic=unwind"` などで上書きしてください。

### クロスコンパイル

主要ターゲットは `rust-toolchain.toml` のツールチェーンに合わせて `rustup target add` で追加します。
必要なクロスツールチェーン（例: `x86_64-w64-mingw32-gcc`）を各OSのパッケージマネージャで用意したうえで、
以下を参考にしてください。

- `CROSS_TARGETS="x86_64-unknown-linux-gnu x86_64-pc-windows-gnu" ./setup.sh`
  まとめてターゲットを追加します（値は環境に合わせて変更してください）
- `.cargo/config.toml` は Windows GNU 向けの `linker`/`ar` のみを設定しています。
  ツール名が異なる場合は適宜更新してください
- ビルド例: `cargo build --target x86_64-unknown-linux-gnu` /
  `cargo build --target x86_64-pc-windows-gnu`
- macOSバイナリはmacOS上でのビルドを推奨します（非macOSからのApple SDKクロスコンパイルは未対応）

### 利用可能な操作

- ファイル編集・作成
- プロジェクト構造の設計
- ドキュメント作成
- タスク管理

### CI依存の操作

- ローカルRustが1.92未満の場合のビルド確認（`cargo build`）
- ローカルRustが1.92未満の場合のテスト実行（`cargo test`）
- ローカルRustが1.92未満の場合のLintチェック（`cargo clippy`）
- ローカルRustが1.92未満の場合のフォーマット確認（`cargo fmt`）

## 対処方針

`rust-toolchain.toml`の指定に合わせてローカルRustを更新し、可能な範囲はローカルで確認します。最終的な品質保証はGitHub Actionsで行います。

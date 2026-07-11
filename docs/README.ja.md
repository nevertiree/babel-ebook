# BabelEbook

[![CI][ci-badge]][ci-url]
[![License: MIT][license-badge]][license-url]
[![Rust Version][rust-badge]][rust-url]
[![Release][release-badge]][release-url]

**BabelEbook**は、大規模言語モデル（LLM）を活用したEPUB翻訳ツールです。原文の言語と訳文の言語を
組み合わせたバイリンガル電子書籍を生成し、各段落の翻訳後に原文を併記します。

他の言語で読む: [简体中文](README.md)

[ci-badge]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml
[license-badge]: https://img.shields.io/badge/License-MIT-yellow.svg
[license-url]: ../LICENSE
[rust-badge]: https://img.shields.io/badge/rust-1.88%2B-blue.svg
[rust-url]: https://www.rust-lang.org/
[release-badge]: https://img.shields.io/github/v/release/nevertiree/babel-ebook
[release-url]: https://github.com/nevertiree/babel-ebook/releases

> EPUBの内容とAPIキーは、あなた自身のコンピュータ上でのみ処理され、プロジェクトメンテナーのサーバーに
> 送信されることはありません。
>
> 他の言語で読む：
> [中文](README.md) · [English](README.en.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Русский](README.ru.md)
> [Español](README.es.md)

<p align="center">
  <img src="assets/screenshots/01-translate.png" alt="BabelEbook メイン画面" width="800">
</p>

<p align="center">
  <a href="https://github.com/nevertiree/babel-ebook/releases/download/v0.4.0/BabelEbook_0.4.0_x64-setup.exe">
    <img alt="Windows版をダウンロード" src="https://img.shields.io/badge/Windows-Download-blue?logo=windows&logoColor=white">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/releases/download/v0.4.0/BabelEbook_0.4.0_amd64.AppImage">
    <img alt="Linux版をダウンロード" src="https://img.shields.io/badge/Linux-Download-orange?logo=linux&logoColor=white">
  </a>
</p>

---

## なぜBabelEbookを選ぶのか？

| 機能 | BabelEbook | オンライン翻訳サービス | Calibreプラグイン |
|---------|------------|--------------------|-----------------|
| 完全ローカル：EPUBをアップロードしない | ✅ | ❌ | ✅ |
| バイリンガル並列レイアウト | ✅ | 一部対応 | 手動調整が必要 |
| ワンクリックデスクトップインストーラー | ✅ | インストール不要 | Calibreが必要 |
| DeepSeek / OpenAI / Anthropic / Ollama | ✅ | 固定ベンダー | プラグイン依存 |
| 用語集、除外セレクター、並列実行 | ✅ | 一部対応 | プラグイン依存 |

---

## スクリーンショット

| メイン画面 | プロバイダー | 翻訳オプション |
|-------------|------------------|---------------------|
| ![メイン画面](assets/screenshots/01-translate.png) | ![計算設定](assets/screenshots/02-settings-compute.png) | ![翻訳オプション](assets/screenshots/03-settings-translation.png) |

| 翻訳の進捗 | ログ |
|----------------------|------|
| ![翻訳の進捗](assets/screenshots/06-translate-progress.png) | ![ログ](assets/screenshots/07-logs-progress.png) |

---

## 対応プラットフォーム

デスクトップGUIは以下の環境で利用できます。

- **Windows**（推奨）: `.exe`（NSIS）および`.msi`インストーラー。
- **Linux**: `.AppImage`（ポータブル、ダブルクリックで実行）およびDebian/Ubuntu系ディストリビューション用の`.deb`
  パッケージ。

macOSには現在、公式のデスクトップインストーラーはありません。macOSユーザーはソースからコマンドライン版を
ビルドして実行できます。

---

## ユーザーガイド

### ダウンロードとインストール

1. [Releases](https://github.com/nevertiree/babel-ebook/releases)ページを開きます。
2. お使いのシステム用のインストーラーをダウンロードします。

   **Windows**
   - **ほとんどのユーザーに推奨**: `BabelEbook_<version>_x64-setup.exe`
     （NSISインストーラー、システム言語に自動対応）。
   - **IT管理者またはサイレント配信用**: `BabelEbook_<version>_x64_en-US.msi`（MSIインストーラー）。

   **Linux**
   - **ほとんどのディストリビューションに推奨**: `BabelEbook_<version>_amd64.AppImage`
     （インストール不要、`chmod +x`を実行してからダブルクリック）。
   - **Debian / Ubuntu**: `BabelEbook_<version>_amd64.deb`
     （ダブルクリックでインストール、または`sudo dpkg -i BabelEbook_<version>_amd64.deb`を実行）。

3. インストーラーをダブルクリックし、指示に従ってください。

> **Linuxでの中国語フォント表示:** Linuxシステムに中国語フォントがインストールされていない場合、UI内の
> 中国語文字が四角（豆腐）で表示されることがあります。Debian/Ubuntuでは`fonts-noto-cjk`など、
> システム推奨の中国語フォントパッケージをインストールしてください。
>
> ```bash
> sudo apt-get install fonts-noto-cjk
> ```

### はじめて使う

#### 1. APIキーを準備する

BabelEbookは、サードパーティ製の大規模言語モデルAPIを呼び出す必要があります。現在、DeepSeek、OpenAI、
Anthropic、およびローカルホストのOllamaに対応しています。

例としてDeepSeekを使用する場合:

1. [DeepSeekプラットフォーム](https://platform.deepseek.com/)にアクセスし、アカウントを作成してAPIキーを
   発行してください。
2. BabelEbookを開き、**設定** → **プロバイダー**を選択してください。
3. プロバイダーに`DeepSeek`を選択し、APIキーを入力してください。
4. **接続テスト**をクリックして接続を確認してください。

> ローカルのOllamaを使用する場合、APIキーは不要です。Base URL（例：`http://localhost:11434`）を入力する
> だけです。

### 本を翻訳する

1. メイン画面で**EPUBを選択**をクリックし、翻訳したい電子書籍を選んでください。
2. 翻訳先言語を選択してください（既定では簡体中国語の`zh-CN`）。
3. **翻訳開始**をクリックしてください。
4. 出力ファイルは、指定した場所に保存されます。

### 共通設定

| 設定項目 | 説明 |
|---------|-------------|
| プロバイダー | 使用するLLMベンダーとAPIキーを選択・入力します。 |
| ターゲット言語 | 翻訳先の言語です。例：`zh-CN`、`en`、`ja`など。 |
| 出力モード | `bilingual`（原文＋訳文）、`translation_only`（訳文のみ）、`interleaved`（段落を交互に配置）。 |
| 並列数 | 同時に翻訳する章の数です。値が大きいほど速くなりますが、コストも増えます。 |
| 最大入出力トークン数 | 1リクエストあたりの最大トークン数です。通常はデフォルト値のままで問題ありません。 |
| 除外セレクター | 翻訳から除外する要素です。例：`.code`、`pre`。 |
| 用語集 | 固有名詞などの訳を固定するための用語表です。 |

### 出力モード

- **バイリンガル（対訳）**: 各翻訳段落の後に原文を併記します。語学習得に最適です。
- **翻訳のみ**: 翻訳後の内容のみを保持します。
- **交互（インターリーブ）**: 原文と訳文の段落が交互に配置されます。

### UI言語

デスクトップアプリはEnglish、Español、日本語、한국어、Русский、简体中文に対応しています。UI言語は初回起動時に
システム言語に基づいて自動選択され、設定から変更できます。

### よくある質問

**Q: 翻訳結果が空、または章が抜けているのはなぜですか？**
A: EPUBの内容がスキャン画像になっていないか確認してください。スキャン画像の場合は、先にOCRを実行して
ください。また、`除外セレクター`を調整して、翻訳しない要素をスキップすることもできます。

**Q: 翻訳にはどれくらいのトークンが必要ですか？**
A: メイン画面またはCLIの**ドライラン**モードを使うと、APIを実際に呼び出さずにトークン数をカウントできます。

**Q: APIキーは安全ですか？**
A: はい。APIキーは既定でWindows Credential Managerに保存され、平文の設定ファイルには保存されません。

---

## 開発者向けガイド

### プロジェクト概要

BabelEbookはRust＋TypeScriptのアーキテクチャを採用しています。

- **Rustコア**（`crates/babel-ebook`）: EPUBの解析、チャンク化、キャッシュ、LLM呼び出し。
- **Rust CLI**（`crates/babel-ebook-cli`）: コマンドラインエントリーポイント。
- **Tauriデスクトップアプリ**（`desktop/`）: Rustバックエンド＋React/TypeScriptフロントエンド。

### 動作環境

- [Rust](https://rustup.rs/) 1.88以降
- [pnpm](https://pnpm.io/) 9以上（デスクトップ開発用）
- Windows 10/11（デスクトップGUI開発用）
- 選択したベンダーのAPIキー

### クイックスタート

```bash
# リポジトリをクローン
git clone https://github.com/nevertiree/babel-ebook.git
cd babel-ebook

# Rustワークスペースをビルド＆テスト
cargo build --workspace
cargo test --workspace

# デスクトップフロントエンドの依存関係をインストール
cd desktop
pnpm install

# デスクトップ開発サーバーを起動
pnpm tauri dev
```

### プロジェクト構成

```text
├── Cargo.toml              # ワークスペース版（信頼できる唯一の情報源）
├── crates/
│   ├── babel-ebook/        # コア翻訳ライブラリ（Rust）
│   └── babel-ebook-cli/    # コマンドラインインターフェース（Rust）
├── desktop/
│   ├── src/                # React + i18nextフロントエンド（TypeScript）
│   ├── src-tauri/          # Tauri Rustバックエンド
│   ├── e2e/                # Playwright GUIテスト
│   └── scripts/            # ビルド＆リリース支援スクリプト
└── release/v<x.y.z>/       # 最終配布用インストーラー（生成物）
```

### ビルドコマンド

#### CLI

```bash
cargo build --release -p babel-ebook-cli
# 出力: target/release/babel-ebook
```

#### Windowsデスクトップインストーラー

```bash
cd desktop
pnpm install
pnpm tauri build
```

出力:

- MSI: `target/release/bundle/msi/BabelEbook_<version>_x64_en-US.msi`
- NSIS: `target/release/bundle/nsis/BabelEbook_<version>_x64-setup.exe`

#### Linuxデスクトップインストーラー

Debian/Ubuntuまたは互換ディストリビューションでは、先にTauriの依存関係をインストールしてください。

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev xdg-utils
```

その後、ビルドします。

```bash
cd desktop
pnpm install
pnpm tauri build
```

出力:

- AppImage: `target/release/bundle/appimage/BabelEbook_<version>_amd64.AppImage`
- deb: `target/release/bundle/deb/BabelEbook_<version>_amd64.deb`

> **Linuxでの中国語UIフォント:** Linuxシステムに中国語フォントがインストールされていない場合、UI内の
> 中国語文字が四角（豆腐）で表示されることがあります。`fonts-noto-cjk`（Debian/Ubuntu:
> `sudo apt-get install fonts-noto-cjk`）または別のシステム中国語フォントをインストールしてください。

### 品質ゲート

PRを作成する前に、以下がすべて通過していることを確認してください。

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd desktop
pnpm exec tsc --noEmit
pnpm build
```

### コントリビューションガイドライン

コントリビューションを歓迎します！
まず[.github/CONTRIBUTING.md](.github/CONTRIBUTING.md)、
[.github/CODE_OF_CONDUCT.md](.github/CODE_OF_CONDUCT.md)、
[.github/SECURITY.md](.github/SECURITY.md)をお読みください。

#### ブランチモデル

このプロジェクトは**Git Flow**に従います。

- `master`: リリース済みの本番コード。
- `develop`: 日次統合のベースライン。
- `release/<version>`: リリース安定化ブランチ。
- `feature/<name>`: 機能ブランチ。

#### コミットスタイル

- [Conventional Commits](https://www.conventionalcommits.org/)を使用してください。
  - `feat:` 新機能
  - `fix:` バグ修正
  - `docs:` ドキュメント更新
  - `refactor:` リファクタリング
  - `chore:` ビルド/ツール/その他
- コミットは小さく、目的を絞ってください。
- APIキー、個人のパス、内部計画書類をコミットしないでください。

#### PRの要件

1. すべてのCIチェックが通過していること。
2. ユーザーに影響する動作が変更される場合は、`docs/README.md`と`CHANGELOG.md`を更新すること。
3. diffは機能または修正の範囲に絞ること。
4. デスクトップの変更には、Playwright E2Eテストを追加または更新すること。

### リリースワークフロー

```bash
cd desktop

# 1. バージョンを更新（patch / minor / major）、Cargo.toml/package.json/tauri.conf.jsonを同期し、タグを作成
pnpm version:bump minor

# 2. タグコミットでフルビルドを実行
pnpm release:build
```

最終成果物は`release/v<version>/`にコピーされます。

### 高度なCLI利用法

```bash
export DEEPSEEK_API_KEY=sk-...

cargo run --release -p babel-ebook-cli -- input.epub -o output.epub \
  --provider deepseek \
  --model deepseek-chat \
  --concurrency 3 \
  --max-input-tokens 4000 \
  --max-output-tokens 2000

# APIを呼び出さずにトークン数を見積もる
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub --dry-run

# JSON設定ファイルを使用する
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub --config config.json
```

CLI引数の一覧については、`babel-ebook --help`を実行してください。

### 対応LLMプロバイダー

| プロバイダー | `--provider` | デフォルトモデル | Base URL | 備考 |
|----------|--------------|---------------|----------|-------|
| DeepSeek | `deepseek` | `deepseek-chat` | `https://api.deepseek.com` | 推奨の既定 |
| OpenAI | `openai` | — | `https://api.openai.com/v1` | `--model`の明示が必要 |
| Anthropic | `anthropic` | `claude-3-5-sonnet-20241022` | `https://api.anthropic.com` | — |
| Ollama | `ollama` | `llama3` | local | APIキー不要 |
| OpenAI-compatible | `openai-compatible` | — | `base_url`で設定 | セルフホストまたはプロキシエンドポイント用 |

### セキュリティ

- **APIキーを決してコミットしないでください:**
  - 環境変数、OSのキーリング、または`.gitignore`で無視されるローカル設定ファイルを使用してください。
  - APIキーをコードに書き込んだり、Gitにコミットしたりしないでください。
- セキュリティ脆弱性は[.github/SECURITY.md](.github/SECURITY.md)を通じて非公開で報告してください。

### 謝辞

Rust、Tauri、React、i18nextで構築されています。

## ライセンス

MIT

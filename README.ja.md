<div align="center">

<img src="docs/brand/branchcut-icon.svg" alt="Branchcut icon" width="112">

# Branchcut

**クエリをコンパイルし、ツリーを枝刈り。**

<p>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2024-DEA584?style=for-the-badge&amp;logo=rust&amp;logoColor=white&amp;labelColor=0A1220" alt="Rust 2024" /></a>
  <img src="https://img.shields.io/badge/core_crates-zero-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="Zero third-party crates in the core" />
  <a href="https://github.com/codex-mohan/branchcut/actions/workflows/install-matrix.yml"><img src="https://img.shields.io/github/actions/workflow/status/codex-mohan/branchcut/install-matrix.yml?style=for-the-badge&amp;label=installers&amp;labelColor=0A1220" alt="Installer checks" /></a>
  <a href="https://github.com/codex-mohan/branchcut/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-BB86FC?style=for-the-badge&amp;labelColor=0A1220" alt="MIT license" /></a>
  <a href="https://github.com/codex-mohan/branchcut/stargazers"><img src="https://img.shields.io/github/stars/codex-mohan/branchcut?style=for-the-badge&amp;labelColor=0A1220&amp;color=FFD700" alt="GitHub stars" /></a>
  <a href="https://github.com/codex-mohan/branchcut/pulls"><img src="https://img.shields.io/badge/PRs-welcome-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="PRs welcome" /></a>
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="README.es.md">Español</a> ·
  <a href="README.fr.md">Français</a> ·
  <a href="README.zh-CN.md">简体中文</a> ·
  <strong>日本語</strong> ·
  <a href="README.hi.md">हिन्दी</a>
</p>

</div>

Branchcut は、日常の検索、開発作業、スクリプトに使えるネイティブのファイルシステムクエリエンジンです。パターン、除外条件、無視ルール、エントリの絞り込み条件、停止条件を一つの走査計画にコンパイルします。結果が見つかる可能性のないディレクトリをスキップし、マッチしたパスを逐次出力し、実行した処理を説明できます。

ライブラリと CLI は、役割ごとに分けた Rust モジュールで構成されています。コアエンジンと通常の CLI は Rust の標準ライブラリのみを使用し、実行時の依存グラフにサードパーティのクレートは含まれません。

[ウェブサイトとドキュメント](https://codex-mohan.github.io/branchcut/) · [リリース](https://github.com/codex-mohan/branchcut/releases) · [機能一覧](docs/reference/feature-status.md) · [バグを報告](https://github.com/codex-mohan/branchcut/issues)

## インストール

[GitHub Releases](https://github.com/codex-mohan/branchcut/releases/latest) からお使いのシステム用のアーカイブをダウンロードし、`SHA256SUMS` と照合して検証します。展開後、実行ファイルを `PATH` が通っているディレクトリに配置します。ビルド済みバイナリを実行するには Rust や Cargo のインストールは不要です。詳しくは[アーカイブからのインストール手順](docs/contributing/releases.md#verify-and-install-an-archive)を参照してください。

Cargo がない環境では、インストールスクリプトで Rust もセットアップできます。Cargo で直接インストールする場合は、Rust ツールチェーンが必要です。以下の方法はいずれも Branchcut をソースからビルドします。

**Windows PowerShell**

```powershell
irm https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.ps1 | iex
```

**Linux と macOS**

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.sh | sh
```

**Cargo**

```sh
cargo install --locked --git https://github.com/codex-mohan/branchcut.git
```

コマンドを実行できることを確認します。

```sh
branchcut --version
branchcut --help
```

スクリプトがインストールするのはリリースビルドです。インストール先の指定、アンインストール、PATH のトラブルシューティングについては、[インストールガイド](docs/getting-started/installation.md)を参照してください。

## 使い方

カレントディレクトリ以下の Rust ソースファイルを探します。

```sh
branchcut --glob 'src/**/*.rs'
```

プレーンテキストを指定すると、入力した文字列をそのままファイル名の検索に使います。

```sh
branchcut config
```

パターン、除外条件、ファイルの絞り込み条件、結果件数の上限を組み合わせます。

```sh
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100
```

走査計画と実際の走査で計測したカウンター値を表示します。

```sh
branchcut --glob '**/*.rs' --explain
branchcut --glob '**/*.rs' --stats
```

### 日常の検索例とクエリの保存

無視ルールを適用しつつ、隠しパスも含めて一般的なソースファイルや設定ファイルを検索します。

```sh
branchcut --glob '**/*.{rs,toml}' --ignore --hidden
```

ファイルサイズと更新時刻で検索します。

```sh
branchcut --glob '**/*' --size '>=1.5MiB' --modified-within 30d
branchcut --glob '**/*.log' --modified-before '2025-01-01T12:00:00+05:30'
```

繰り返し使うプロジェクトのクエリを `.branchcut-queries` に保存します。

```text
[workspace]
glob packages/**/src/**/*.{rs,ts}
exclude **/target/**
ignore true
max-depth 8
```

保存したプロファイルを実行します。

```sh
branchcut --config .branchcut-queries --profile workspace --explain
```

ほかの例は、[日常の検索](docs/guides/everyday-search.md)、[無視ルールと隠しパス](docs/guides/ignore-and-hidden.md)、[プロジェクトクエリの保存](docs/guides/project-queries.md)を参照してください。

### パイプラインとコマンド

任意の Unix ファイル名を別のプログラムに渡すには、NUL 区切りのパスを使います。

```sh
branchcut --glob '**/*.rs' --print0 | xargs -0 rustfmt
```

後続のツールが構造化されたパスを必要とする場合は JSON Lines を使えます。ファイルシステムのメタデータは `--json-metadata` で追加できます。

```sh
branchcut --glob '**/*.log' --json-metadata
```

各結果を引数として直接コマンドに渡し、シェルによるパスの解釈を介さずに実行します。

```sh
branchcut --glob '**/*.tmp' --exec 'rm {}'
```

結果が多い場合は、`--exec-batch` でパスを上限付きのグループにまとめて実行できます。`--exec-preview` を使うと、コマンドを実行せずに引数リストを確認できます。プレースホルダー、プラットフォームごとの詳細、失敗時の動作については、[出力とコマンド実行](docs/guides/output-and-exec.md)を参照してください。

## 機能

- **Glob 検索**では、`*`、`?`、`**`、文字クラス、入れ子の波括弧、上限のある範囲指定、単一のパスコンポーネント内の extglob が使えます。
- **クエリの組み合わせ**では、検索対象を指定する複数のパターン、除外条件、拡張子と種類の選択、深さ、リテラル文字列、メタデータによる絞り込みを一つのクエリにまとめられます。
- **無視ルール**では、隠しパスが既定で除外されます。Git の無視ルール、`.ignore`、明示的に指定した無視ファイルは必要に応じて有効にできます。
- **走査の制御**には、逐次出力、`--first`、`--limit`、ソート、ワーカー数の指定、シンボリックリンクの追跡、Unix での単一ファイルシステム内の走査、同時に開くディレクトリ数の制限が含まれます。
- **メタデータによる絞り込み**には、10 進単位または 2 進単位の正確なファイルサイズしきい値と、更新時刻の範囲を指定できます。クエリでメタデータが必要な場合にのみ取得します。
- **再利用可能なクエリ**は、コンパクトなテキスト設定ファイルに名前付きのプロジェクトクエリプロファイルとして保存できます。
- **診断**には、計画を表示する `--explain`、候補パスの判定結果を示す `--why PATH`、実測した走査の統計を表示する `--stats` が使えます。
- **出力とワークフロー**では、プレーンなパス、任意でメタデータを含められる JSON Lines、NUL 区切りのパス、出力書式、色、ハイパーリンク、シェルを介さないコマンド実行、上限付きの一括実行、シェル補完を利用できます。
- **ライブラリ API** では、`Query` を一度コンパイルすれば、CLI を呼び出さずに Rust ライブラリから結果を順に処理できます。

構文、制限、実装状況については、[CLI リファレンス](docs/reference/cli.md)、[Glob 構文](docs/reference/glob-syntax.md)、[機能の実装状況](docs/reference/feature-status.md)を参照してください。

## 仕組み

複数の条件を含む一般的な検索は、ツリー全体を走査し、glob マッチング、除外判定、メタデータの読み取り、結果の収集を行う方法で実装できます。Branchcut はこれらの処理を一つのクエリ計画にコンパイルし、マッチする可能性が残っているパターンの状態をツリーの走査に引き継ぎます。

```text
patterns + exclusions + ignores + filters + termination
                          │
                          ▼
             compiled traversal plan
       narrowed root · active pattern states
       pruning rules · metadata requirements
                          │
                          ▼
             stream matching filesystem paths
```

この設計は、ファイルシステムに対する不要な処理を減らすことを目指しています。すべてのクエリが他のあらゆるツールより速くなる保証はありません。以下の実測結果と、記録した測定方法を参照してください。

## 性能

この表は、コミット `8035ad6` の検索実装を対象に、2026 年 11 月 18 日に Windows/NTFS で実施した**既定設定での CLI 実行**の結果です。各ツールは同じ 100,000 ファイルのテストデータを検索し、計測の前後に結果集合を照合しました。3 回のウォームアップ後、毎回新しいプロセスで 9 回実行した中央値です。プロセスの起動と出力の取得も計測時間に含まれます。

`n/t` は、同等の実行条件で計測しなかったことを示します。機能の有無はこの表記からは判断できません。ripgrep は `--files` モードで計測しており、ファイル内容の検索は行っていません。

| ワークロード | Branchcut | zlob | ripgrep (`rg --files`) | fd | fast-glob | tinyglobby |
|---|---:|---:|---:|---:|---:|---:|
| 広範囲のファイル検索 | **68.79 ms** | 123.94 ms | 109.17 ms | 117.14 ms | 370.45 ms | 242.30 ms |
| 固定プレフィックス配下の Rust ファイル | 40.34 ms | **35.46 ms** | 75.23 ms | 40.91 ms | 210.08 ms | 153.56 ms |
| 除外条件付きの Rust/TOML ファイル | **59.27 ms** | n/t | 79.27 ms | 92.57 ms | 333.23 ms | 208.11 ms |
| 隠しパスを含む Rust ファイル | **64.51 ms** | 98.29 ms | 84.53 ms | 94.45 ms | 314.34 ms | 195.53 ms |
| `.gitignore` を適用した Rust ファイル | **68.29 ms** | 80.07 ms | 77.18 ms | 113.49 ms | n/t | n/t |
| ディレクトリ | **53.76 ms** | 80.35 ms | n/t | 84.66 ms | 251.20 ms | 152.86 ms |
| Rust ファイルをソートして出力 | **101.96 ms** | 103.99 ms | 260.04 ms | n/t | n/t | n/t |
| 最初の 10 件の Rust ファイル | **37.50 ms** | 98.46 ms | n/t | 46.00 ms | n/t | n/t |
| 広範囲の検索で NUL 区切り出力 | **72.61 ms** | n/t | 105.54 ms | 109.36 ms | n/t | n/t |
| Rust ファイルをファイル名で再帰検索 | **64.26 ms** | 96.60 ms | 76.10 ms | 94.15 ms | 318.07 ms | 208.26 ms |

固定プレフィックスのクエリでは、既定設定で zlob が最速でした。同じクエリで、Branchcut は 4 ワーカーで 33.39 ms を記録しました。このテストデータの残りのワークロードでは、既定設定の Branchcut が最速で、ソートありの結果は zlob と僅差でした。測定結果は、ここで計測したワークロードに限られます。広範囲のファイル検索では、既定設定の Branchcut のピークワーキングセットは 6.38 MiB でした。すべてのツールと、検証したすべてのワーカー構成について、メモリのサンプルを保存しています。

- [既定設定と調整後の全結果、メモリ使用量、測定方法](docs/evidence/current-cli-comparison.md)
- [11 月 18 日の測定生データ](benchmarks/results/2026-11-18-gap-closure-cli.json)
- [11 月 12 日の過去の比較](docs/evidence/cli-comparison.md)
- [ベンチマークの方法と制限](BENCHMARKS.md)

## Rust ライブラリ

Rust ライブラリを使うと、組み込み先のアプリケーションでクエリを一度コンパイルし、パスを見つかるたびにコールバックに渡せます。

```rust
use branchcut::{Completion, Query};
use std::{io, ops::ControlFlow, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = Query {
        patterns: vec!["src/**/*.rs".into()],
        ..Query::default()
    }
    .compile()?;

    let report = query.visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
        println!("{}", path.display());
        Ok(ControlFlow::Continue(()))
    })?;

    assert_eq!(report.completion, Completion::Complete);
    Ok(())
}
```

コールバックは `ControlFlow::Break` を返して走査を停止できます。エラー処理、ソート、ビジターの実装例については、[Rust ライブラリのチュートリアル](docs/tutorials/rust-library.md)を参照してください。

## 互換性と制限

Branchcut はファイルシステムのクエリに特化したツールです。fast-glob、tinyglobby、zlob、fd、ripgrep の API 全体をそのまま置き換えることはできません。互換性の対象は、仕様を文書化し、動作を確認した範囲に限られます。

- 無視ルールは明示的に有効にします。`--gitignore` は Git の無視ルールと入れ子の `.gitignore` を有効にし、`--ignore` は入れ子の `.ignore` を追加します。明示的な `--ignore-file` のルールが最優先です。Git 設定の一部の条件や無視ルールの一部の構文には対応していません。
- 既定ではバイト単位でマッチングします。`--unicode` を指定すると、有効な UTF-8 パスに対して Unicode スカラー値単位のワイルドカードと、単純な小文字化による大文字・小文字の照合が有効になります。完全なケースフォールディング、正規化、書記素単位のマッチングには対応していません。
- 10 進表記のサイズしきい値は正確に計算され、バイト数に換算した値は整数である必要があります。更新日時には、UTC の日付、オフセットや小数秒を含む RFC3339 の日時、Unix 時刻の `@seconds` 表記を指定できます。
- ディレクトリへのシンボリックリンクは既定で追跡しません。追跡を有効にした場合は、循環を検出しながら逐次走査します。`--one-file-system` は Unix 専用です。
- `--max-open` に有限値を指定すると、未処理のエントリをメモリに一時保存し、保存した走査状態から再開することで、同時に開くディレクトリハンドル数を制限します。開いておくハンドルが減る一方で、メモリ使用量は増えます。
- ファイル内容の検索、インデックスの維持、変更の監視、GUI の提供は行いません。

- `--strict` を指定しない場合、ファイルシステムのエラーを報告しつつ検索を続行します。ソートする場合は走査を終えてから出力し、通常のソートなしのクエリは結果を逐次出力します。

## ドキュメント

| はじめに | リファレンスと応用 |
|---|---|
| [クイックスタート](docs/getting-started/quick-start.md) | [CLI リファレンス](docs/reference/cli.md) |
| [日常の検索例](docs/guides/everyday-search.md) | [Glob 構文](docs/reference/glob-syntax.md) |
| [クエリの保存](docs/guides/project-queries.md) | [無視ルールと隠しパス](docs/guides/ignore-and-hidden.md) |
| [Rust ライブラリのチュートリアル](docs/tutorials/rust-library.md) | [出力とコマンド実行](docs/guides/output-and-exec.md) |
| [機能の実装状況](docs/reference/feature-status.md) | [互換性リファレンス](docs/reference/compatibility.md) |

上の言語リンクから、この README の各言語版に移動できます。詳細ドキュメントは現在英語で提供しています。正式なドキュメントは [website/](website/README.md) のサイトにも使用されています。

## コントリビューション

バグ報告、対象を絞ったプルリクエスト、翻訳の修正を歓迎します。動作の退行が確認された場合は、必ずテストを追加してください。性能に関する主張には再現可能な測定が必要で、性能が他より劣った結果も保存してください。

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo tree -e normal
```

[コントリビューター向けの検証ガイド](docs/contributing/verification.md)、[アーキテクチャのルール](AGENTS.md)、[ロードマップ](ROADMAP.md)、[ドキュメントガイド](docs/contributing/documentation.md)、[ベンチマークの測定方法](BENCHMARKS.md)を参照してください。ウェブサイトは、外部クレートを含まない Rust の依存グラフから独立しています。

## 成り立ちとライセンス

バージョンを付けた[コンテスト提出版](https://github.com/codex-mohan/branchcut/tree/submission-v1)は、当時の記録として残しています。Branchcut は Zero Dependency 72-Hour Hackathon のために開発されました。現在の開発は [AGENTS.md](AGENTS.md) のコンテスト終了後の製品・アーキテクチャ方針に従い、提出版のタグは書き換えません。

Branchcut は [MIT ライセンス](LICENSE)で提供しています。依存関係の一覧は [STDLIB.md](STDLIB.md) を参照してください。

<div align="center">

<img src="docs/brand/branchcut-icon.svg" alt="Branchcut icon" width="112">

# Branchcut

**编译查询，剪枝遍历。**

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
  <strong>简体中文</strong> ·
  <a href="README.ja.md">日本語</a> ·
  <a href="README.hi.md">हिन्दी</a>
</p>

</div>

Branchcut 是一款原生文件系统查询引擎，适用于日常搜索、开发工作流和脚本。它将模式、排除条件、忽略规则、条目筛选条件和停止条件编译为一个遍历计划。它可以跳过不可能产生结果的目录，以流式方式返回匹配路径，并说明实际执行了哪些操作。

库和命令行界面由职责明确的 Rust 模块组成。核心引擎和常规命令行工具只使用 Rust 标准库，运行时依赖图中没有第三方 crate。

[网站和文档](https://codex-mohan.github.io/branchcut/) · [发布版本](https://github.com/codex-mohan/branchcut/releases) · [功能清单](docs/reference/feature-status.md) · [报告缺陷](https://github.com/codex-mohan/branchcut/issues)

## 安装

从 [GitHub Releases](https://github.com/codex-mohan/branchcut/releases/latest) 下载适用于你系统的归档文件，根据 `SHA256SUMS` 校验文件，解压后将可执行文件放到 `PATH` 中的目录里。预编译二进制文件无需 Rust 或 Cargo。请参阅[归档安装说明](docs/contributing/releases.md#verify-and-install-an-archive)。

缺少 Cargo 时，安装脚本可以先安装 Rust。直接使用 Cargo 安装则需要已有的 Rust 工具链。以下每种方法都从源代码构建 Branchcut。

**Windows PowerShell**

```powershell
irm https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.ps1 | iex
```

**Linux 和 macOS**

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.sh | sh
```

**Cargo**

```sh
cargo install --locked --git https://github.com/codex-mohan/branchcut.git
```

验证命令是否可用。

```sh
branchcut --version
branchcut --help
```

脚本安装的是发布版构建。有关自定义安装位置、卸载和 PATH 故障排查的信息，请参阅[安装指南](docs/getting-started/installation.md)。

## 用法

在当前目录下查找 Rust 源文件。

```sh
branchcut --glob 'src/**/*.rs'
```

输入纯文本时，按字面文本搜索文件名。

```sh
branchcut config
```

将模式、排除条件、文件筛选条件和结果数量上限组合起来。

```sh
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100
```

查看查询计划和实际遍历计数。

```sh
branchcut --glob '**/*.rs' --explain
branchcut --glob '**/*.rs' --stats
```

### 日常示例和保存的查询

查找常见源文件和配置文件，同时应用忽略规则并包含隐藏路径。

```sh
branchcut --glob '**/*.{rs,toml}' --ignore --hidden
```

按大小和修改时间搜索。

```sh
branchcut --glob '**/*' --size '>=1.5MiB' --modified-within 30d
branchcut --glob '**/*.log' --modified-before '2025-01-01T12:00:00+05:30'
```

在 `.branchcut-queries` 中保存可反复使用的项目查询。

```text
[workspace]
glob packages/**/src/**/*.{rs,ts}
exclude **/target/**
ignore true
max-depth 8
```

然后运行该配置。

```sh
branchcut --config .branchcut-queries --profile workspace --explain
```

更多示例请参阅[日常搜索](docs/guides/everyday-search.md)、[忽略规则和隐藏路径](docs/guides/ignore-and-hidden.md)和[保存项目查询](docs/guides/project-queries.md)。

### 管道和命令

向其他程序传递任意 Unix 文件名时，请使用 NUL 分隔的路径。

```sh
branchcut --glob '**/*.rs' --print0 | xargs -0 rustfmt
```

下游工具需要结构化路径时，可以使用 JSON Lines，也可以用 `--json-metadata` 添加文件系统元数据。

```sh
branchcut --glob '**/*.log' --json-metadata
```

将每条结果作为参数传给命令，直接执行，无需由 shell 解释路径。

```sh
branchcut --glob '**/*.tmp' --exec 'rm {}'
```

当结果很多时，`--exec-batch` 会将路径分组，每次调用的规模有上限。使用 `--exec-preview` 可以在不执行命令的情况下查看参数列表。有关占位符、平台细节和失败时的行为，请参阅[输出和执行](docs/guides/output-and-exec.md)。

## 功能

- **Glob 搜索**支持 `*`、`?`、`**`、字符类、嵌套花括号和有界范围，以及单个路径组件内的 extglob。
- **组合查询**可在同一查询中使用多个包含模式、排除条件、扩展名和类型筛选、深度、字面搜索词和元数据筛选条件。
- **内容搜索**可通过 `--content`（别名 `--grep`）在已选中的文件内进行字面量搜索，支持多个模式按 OR 组合、`路径:行:列:文本` 输出、`-l`、`--count`、大小写模式、用 `--text` 强制扫描二进制文件以及提前停止。不支持正则表达式或多行模式。
- **忽略控制**默认排除隐藏路径。可以选择启用 Git 忽略规则来源、`.ignore` 或显式指定的忽略文件。
- **遍历控制**支持流式输出、`--first`、`--limit`、排序、选择工作线程数、跟随符号链接、在 Unix 上仅遍历同一文件系统，以及限制同时打开的目录数。
- **元数据筛选**支持精确的十进制或二进制单位文件大小阈值，以及修改时间范围。查询未要求元数据时会避免读取元数据。
- **可复用查询**可将项目查询保存为小型文本配置文件中的命名配置。
- **诊断**提供 `--explain`、`--why PATH` 和 `--stats`，可查看查询计划、候选路径的判定结果和实测的遍历工作量。
- **输出和工作流**支持纯文本路径、可附带元数据的 JSON Lines、NUL 分隔的路径、格式化、颜色、超链接、无需 shell 的命令执行、有上限的分批执行和 shell 补全。
- **库 API**支持将 `Query` 编译一次，然后通过 Rust 库访问结果，无需调用命令行工具。

有关语法、限制和实现状态，请参阅[命令行参考](docs/reference/cli.md)、[Glob 语法](docs/reference/glob-syntax.md)和[功能状态](docs/reference/feature-status.md)。

## 工作原理

常见的多条件搜索可以先遍历整个目录树，再做 glob 匹配、应用排除条件、读取元数据、收集结果。Branchcut 将这些步骤编译为一个查询计划，并在遍历目录树时只传递仍可能匹配的模式状态。

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

这一设计旨在减少不必要的文件系统操作。并非每个查询都一定比其他所有工具快。请参阅下方的实测对比和记录中的测试方法。

## 性能

下表列出了 2026 年 11 月 18 日在 Windows/NTFS 上对已提交的搜索实现（`8035ad6`）进行测试时，**默认命令行调用**的结果。各工具都搜索同一个含 100,000 个文件的测试目录树，并在计时前后校验结果集。每组数据先预热三次，再在新进程中运行九次并取中位数，耗时包含进程启动和输出收集。

`n/t` 表示测试程序没有测量等效调用，无法据此判断工具是否支持相应功能。ripgrep 使用 `--files` 模式测量，不搜索文件内容。

| 工作负载 | Branchcut | zlob | ripgrep (`rg --files`) | fd | fast-glob | tinyglobby |
|---|---:|---:|---:|---:|---:|---:|
| 全范围文件搜索 | **68.79 ms** | 123.94 ms | 109.17 ms | 117.14 ms | 370.45 ms | 242.30 ms |
| 固定前缀下的 Rust 文件 | 40.34 ms | **35.46 ms** | 75.23 ms | 40.91 ms | 210.08 ms | 153.56 ms |
| 带排除条件的 Rust/TOML 文件 | **59.27 ms** | n/t | 79.27 ms | 92.57 ms | 333.23 ms | 208.11 ms |
| 包含隐藏路径的 Rust 文件 | **64.51 ms** | 98.29 ms | 84.53 ms | 94.45 ms | 314.34 ms | 195.53 ms |
| 应用 `.gitignore` 的 Rust 文件 | **68.29 ms** | 80.07 ms | 77.18 ms | 113.49 ms | n/t | n/t |
| 目录 | **53.76 ms** | 80.35 ms | n/t | 84.66 ms | 251.20 ms | 152.86 ms |
| 排序后的 Rust 文件 | **101.96 ms** | 103.99 ms | 260.04 ms | n/t | n/t | n/t |
| 前 10 个 Rust 文件 | **37.50 ms** | 98.46 ms | n/t | 46.00 ms | n/t | n/t |
| 全范围搜索，NUL 分隔输出 | **72.61 ms** | n/t | 105.54 ms | 109.36 ms | n/t | n/t |
| 递归按文件名搜索 Rust 文件 | **64.26 ms** | 96.60 ms | 76.10 ms | 94.15 ms | 318.07 ms | 208.26 ms |

默认设置下，zlob 在固定前缀查询中最快。Branchcut 用四个工作线程运行同一查询耗时 33.39 ms。在此测试目录树上，Branchcut 在其余默认配置的测试中领先，在排序查询中与 zlob 的结果接近。这些测量数据只反映所测试的工作负载。全范围文件搜索中，默认配置的 Branchcut 实测峰值工作集为 6.38 MiB。每个工具以及所有已测试的工作线程配置均保留了内存采样结果。

- [完整的默认与调优结果、内存数据和测试方法](docs/evidence/current-cli-comparison.md)
- [11 月 18 日的原始结果记录](benchmarks/results/2026-11-18-gap-closure-cli.json)
- [11 月 12 日的历史对比](docs/evidence/cli-comparison.md)
- [基准测试方法和限制](BENCHMARKS.md)

## Rust 库

宿主应用可以通过 Rust 库编译一次查询，然后将路径逐条传给回调。

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

回调可通过 `ControlFlow::Break` 停止遍历。有关错误处理、排序和访问器示例，请参阅 [Rust 库教程](docs/tutorials/rust-library.md)。

## 兼容性和限制

Branchcut 专注于文件系统查询，无法直接替代 fast-glob、tinyglobby、zlob、fd 或 ripgrep 的完整 API。兼容性仅涵盖已记录且经过验证的语义。

- 忽略规则需显式启用。`--gitignore` 启用 Git 忽略规则来源和嵌套的 `.gitignore` 文件，`--ignore` 加入嵌套的 `.ignore` 文件，显式指定的 `--ignore-file` 规则具有最高优先级。部分 Git 配置条件和忽略规则语法尚不支持。
- 默认按字节匹配。对采用有效 UTF-8 编码的路径，`--unicode` 启用基于 Unicode 标量值的通配符，以及通过简单的小写转换进行大小写匹配。它不支持完整的大小写折叠、Unicode 规范化或字素匹配。
- 以十进制表示的大小阈值会精确计算，其结果必须为整数个字节。修改时间支持 UTC 日期、带时区偏移或小数秒的 RFC3339 时间，以及 Unix `@seconds` 时间戳。
- 默认不跟随指向目录的符号链接。启用后，遍历将串行执行并检测循环。`--one-file-system` 仅在 Unix 上可用。
- 将 `--max-open` 设为有限值时，程序会将待处理条目暂存于内存，再从缓冲的遍历状态继续，以额外的内存占用换取更少的打开目录句柄。
- Branchcut 选择路径并读取文件系统元数据。通过 `--content`（别名 `--grep`）可以对已选中的文件进行可选的文本字面量搜索：多个模式按 OR 组合，输出 `路径:行:列:文本`，除非指定 `--text` 否则跳过二进制文件，未请求内容搜索的查询不会读取文件内容。它不支持正则表达式、捕获或多行匹配。项目也不维护索引、不监视文件变化，也不提供图形界面。

- 未启用 `--strict` 时，程序会报告文件系统错误并继续查找。排序查询会先完成扫描再输出，普通的未排序查询则逐条输出结果。

## 文档

| 入门 | 参考和进阶工作流 |
|---|---|
| [快速入门](docs/getting-started/quick-start.md) | [命令行参考](docs/reference/cli.md) |
| [日常示例](docs/guides/everyday-search.md) | [Glob 语法](docs/reference/glob-syntax.md) |
| [保存的查询](docs/guides/project-queries.md) | [忽略规则和隐藏路径](docs/guides/ignore-and-hidden.md) |
| [Rust 库教程](docs/tutorials/rust-library.md) | [输出和执行](docs/guides/output-and-exec.md) |
| [功能状态](docs/reference/feature-status.md) | [兼容性参考](docs/reference/compatibility.md) |

上方的语言链接指向本 README 的各语言译本。详细文档目前为英文。正式文档也用来生成 [website/](website/README.md) 中的网站。

## 贡献

欢迎提交缺陷报告、针对具体问题的拉取请求以及翻译修正。每项确认的行为回归都需要有测试覆盖。性能声明需要可复现的测量数据，并应保留表现落后的测试结果。

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo tree -e normal
```

请阅读[贡献者验证指南](docs/contributing/verification.md)、[架构规则](AGENTS.md)、[路线图](ROADMAP.md)、[文档指南](docs/contributing/documentation.md)和[基准测试方法](BENCHMARKS.md)。网站独立于零第三方依赖的 Rust 依赖图。

## 起源和许可证

带版本标签的[竞赛提交版本](https://github.com/codex-mohan/branchcut/tree/submission-v1)作为历史记录予以保留。Branchcut 最初为 Zero Dependency 72-Hour Hackathon 开发，后续开发遵循 [AGENTS.md](AGENTS.md) 中的赛后产品和架构规则，竞赛提交标签保持原样。

Branchcut 采用 [MIT 许可证](LICENSE)。依赖清单见 [STDLIB.md](STDLIB.md)。

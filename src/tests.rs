use crate::EntryType as WantedType;
use crate::cli::args::Options;
use crate::cli::exec::parse_command_line;
use crate::cli::output::json_escape;
use crate::path::normal_components;
use crate::pattern::{
    Pattern, PatternClass, PatternProgram, Segment, SegmentMatcher, expand_braces,
};
use crate::query::common_literal_prefix;
use crate::{CompiledQuery, Stats};
use std::env;
use std::ffi::OsStr;
#[cfg(unix)]
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// Exercise the same presentation adapter as the CLI while keeping engine tests
// independent of stdout and process-global working directories.
struct QueryPlan {
    core: CompiledQuery,
    options: Options,
}
impl QueryPlan {
    fn compile(options: &Options) -> crate::error::Result<Self> {
        Ok(Self {
            core: options.query().compile()?,
            options: options.clone(),
        })
    }
}
struct Runner<'a, W: Write> {
    plan: &'a QueryPlan,
    writer: Option<W>,
    output: Vec<PathBuf>,
    stats: Stats,
    stopped: bool,
}
impl<'a, W: Write> Runner<'a, W> {
    fn new(plan: &'a QueryPlan, writer: W) -> Self {
        Self {
            plan,
            writer: Some(writer),
            output: Vec::new(),
            stats: Stats::default(),
            stopped: false,
        }
    }
    fn run(&mut self) -> io::Result<()> {
        let mut sink =
            crate::cli::output::Output::new(&self.plan.options, self.writer.take().unwrap())
                .unwrap();
        self.stats = crate::cli::execute(&self.plan.core, &self.plan.options, &mut sink)?;
        self.stopped = sink.stopped
            || self
                .plan
                .options
                .limit
                .is_some_and(|limit| self.stats.matches >= limit as u64);
        self.output = sink.output;
        Ok(())
    }
}
fn run_parallel(
    plan: &QueryPlan,
    threads: usize,
    strict: bool,
) -> io::Result<(Stats, Vec<PathBuf>)> {
    let collected = plan.core.collect_parallel(threads, strict)?;
    Ok((collected.report.stats, collected.paths))
}

fn parts(path: &Path) -> Vec<&OsStr> {
    normal_components(path)
}

fn program_states(program: &PatternProgram, path: &Path) -> Vec<usize> {
    parts(path)
        .into_iter()
        .fold(program.initial_states(), |states, component| {
            program.advance(&states, component)
        })
}

fn matches(pattern: &str, path: &str) -> bool {
    let patterns = [Pattern::compile(pattern.to_owned()).unwrap()];
    let program = PatternProgram::compile(&patterns);
    program.states_match(&program_states(&program, Path::new(path)))
}

#[test]
fn matches_basic_segment_syntax() {
    assert!(matches("src/*.rs", "src/main.rs"));
    assert!(matches("src/ma?n.rs", "src/main.rs"));
    assert!(matches("src/[a-z]ain.rs", "src/main.rs"));
    assert!(matches("src/[!0-9]*.rs", "src/main.rs"));
    assert!(!matches("src/*.rs", "src/nested/main.rs"));
    assert!(!matches("src/[!a-z]*.rs", "src/main.rs"));
}

#[test]
fn classifies_patterns_and_specializes_common_segments() {
    let literal = Pattern::compile("src/main.rs".to_owned()).unwrap();
    let shallow = Pattern::compile("src/*.rs".to_owned()).unwrap();
    let fixed = Pattern::compile("src/**/*.rs".to_owned()).unwrap();
    let unbounded = Pattern::compile("**/*.rs".to_owned()).unwrap();
    let prefix = Pattern::compile("src/test*".to_owned()).unwrap();

    assert_eq!(literal.class, PatternClass::Literal);
    assert_eq!(shallow.class, PatternClass::SingleDirectory);
    assert_eq!(fixed.class, PatternClass::FixedPrefixRecursive);
    assert_eq!(unbounded.class, PatternClass::UnboundedRecursive);
    assert!(matches!(
        shallow.segments.last(),
        Some(Segment::Match(SegmentMatcher::Suffix(suffix))) if suffix == b".rs"
    ));
    assert!(matches!(
        prefix.segments.last(),
        Some(Segment::Match(SegmentMatcher::Prefix(value))) if value == b"test"
    ));
}

#[test]
fn globstar_matches_zero_or_many_components() {
    assert!(matches("src/**/mod.rs", "src/mod.rs"));
    assert!(matches("src/**/mod.rs", "src/a/b/mod.rs"));
    assert!(!matches("src/**/mod.rs", "other/mod.rs"));
}

#[test]
fn brace_expansion_supports_nested_groups_deterministically() {
    assert_eq!(
        expand_braces("**/*.{rs,toml}").unwrap(),
        ["**/*.rs", "**/*.toml"]
    );
    assert!(expand_braces("*.{rs}").is_err());
    assert_eq!(
        expand_braces("*.{rs,{ts,tsx}}").unwrap(),
        ["*.rs", "*.ts", "*.tsx"]
    );
}

#[test]
fn prefix_viability_prunes_impossible_directories() {
    let patterns = [Pattern::compile("packages/*/src/**/*.rs".to_owned()).unwrap()];
    let program = PatternProgram::compile(&patterns);
    assert!(
        program.states_descendant_possible(&program_states(&program, Path::new("packages/core")))
    );
    assert!(program.states_descendant_possible(&program_states(
        &program,
        Path::new("packages/core/src/deep")
    )));
    assert!(!program.states_descendant_possible(&program_states(&program, Path::new("unrelated"))));
}

#[test]
fn shared_program_merges_common_pattern_segments() {
    let patterns = [
        Pattern::compile("src/**/*.rs".to_owned()).unwrap(),
        Pattern::compile("src/**/*.toml".to_owned()).unwrap(),
        Pattern::compile("src/**/test*.rs".to_owned()).unwrap(),
    ];
    let independent_nodes = 1 + patterns
        .iter()
        .map(|pattern| pattern.segments.len())
        .sum::<usize>();
    let program = PatternProgram::compile(&patterns);

    assert!(program.nodes.len() < independent_nodes);
    assert!(program.states_match(&program_states(&program, Path::new("src/deep/lib.rs"))));
    assert!(program.states_match(&program_states(&program, Path::new("src/Cargo.toml"))));
}

#[test]
fn shared_globstar_does_not_leak_into_sibling_patterns() {
    let cases: &[(&[&str], &[&str])] = &[
        (
            &["docs/**/__nonexistent__", "docs/cli.md"],
            &[
                "docs/cli.md",
                "docs/reference/cli.md",
                "docs/reference/__nonexistent__",
                "docs/reference/deep/cli.md",
            ],
        ),
        (
            &["src/**/foo.rs", "src/components/bar.rs"],
            &[
                "src/foo.rs",
                "src/deep/foo.rs",
                "src/components/bar.rs",
                "src/deep/components/bar.rs",
            ],
        ),
        (
            &["src/**/target/**", "src/other/fixed.rs"],
            &[
                "src/target",
                "src/target/generated.rs",
                "src/deep/target/generated.rs",
                "src/other/fixed.rs",
                "src/deep/other/fixed.rs",
            ],
        ),
        (
            &["a/**/**/leaf", "a/fixed.txt"],
            &["a/leaf", "a/deep/leaf", "a/fixed.txt", "a/deep/fixed.txt"],
        ),
        (
            &["build/**", "build.rs"],
            &["build", "build/file.rs", "build/deep/file.rs", "build.rs"],
        ),
    ];

    for &(sources, paths) in cases {
        let patterns = sources
            .iter()
            .map(|source| Pattern::compile((*source).to_owned()).unwrap())
            .collect::<Vec<_>>();
        let program = PatternProgram::compile(&patterns);

        for path in paths {
            let combined = program.states_match(&program_states(&program, Path::new(path)));
            let independent = sources.iter().any(|source| matches(source, path));
            assert_eq!(
                combined, independent,
                "shared patterns {sources:?} disagreed with independent matching for {path:?}"
            );
        }
    }
}

#[test]
fn mixed_globstar_query_does_not_return_or_exclude_sibling_matches() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join("docs/reference")).unwrap();
    fs::write(fixture.0.join("docs/reference/cli.md"), b"").unwrap();

    let positives = Options {
        cwd: fixture.0.clone(),
        positive: vec![
            "docs/**/__nonexistent__".to_owned(),
            "docs/cli.md".to_owned(),
        ],
        sort: true,
        ..Options::default()
    };
    let plan = QueryPlan::compile(&positives).unwrap();
    let mut runner = Runner::new(&plan, Vec::new());
    runner.run().unwrap();
    assert!(runner.output.is_empty());

    let with_exclusions = Options {
        positive: vec!["**/*.md".to_owned()],
        negative: vec![
            "docs/**/__nonexistent__".to_owned(),
            "docs/cli.md".to_owned(),
        ],
        ..positives
    };
    let plan = QueryPlan::compile(&with_exclusions).unwrap();
    let mut runner = Runner::new(&plan, Vec::new());
    runner.run().unwrap();
    assert_eq!(runner.output, [PathBuf::from("docs/reference/cli.md")]);
}

#[test]
fn common_prefix_stops_at_first_difference() {
    let patterns = [
        Pattern::compile("packages/core/src/**/*.rs".to_owned()).unwrap(),
        Pattern::compile("packages/core/tests/**/*.rs".to_owned()).unwrap(),
    ];
    assert_eq!(
        common_literal_prefix(&patterns),
        [b"packages".to_vec(), b"core".to_vec()]
    );
}

#[test]
fn invalid_patterns_return_errors() {
    assert!(Pattern::compile("src/[abc".to_owned()).is_err());
    assert!(Pattern::compile("../*.rs".to_owned()).is_err());
    assert!(Pattern::compile("/tmp/*.rs".to_owned()).is_err());
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("branchcut-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join("src/nested")).unwrap();
        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("src/lib.rs"), b"").unwrap();
        fs::write(root.join("src/nested/config.toml"), b"").unwrap();
        fs::write(root.join("target/debug/generated.rs"), b"").unwrap();
        fs::write(root.join(".hidden/secret.rs"), b"").unwrap();
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn traversal_filters_extensions_hidden_paths_and_excluded_subtrees() {
    let fixture = Fixture::new();
    let options = Options {
        cwd: fixture.0.clone(),
        positive: vec!["**/*.{rs,toml}".to_owned()],
        negative: vec!["**/target/**".to_owned()],
        extensions: vec![b"rs".to_vec()],
        sort: true,
        ..Options::default()
    };
    let plan = QueryPlan::compile(&options).unwrap();
    let mut runner = Runner::new(&plan, Vec::new());
    runner.run().unwrap();

    assert_eq!(runner.output, [PathBuf::from("src/lib.rs")]);
    assert_eq!(runner.stats.matches, 1);
    assert_eq!(runner.stats.dirs_pruned_exclude, 1);
    assert_eq!(runner.stats.metadata_calls, 1);
}

#[test]
fn traversal_honors_early_limit() {
    let fixture = Fixture::new();
    let options = Options {
        cwd: fixture.0.clone(),
        positive: vec!["**/*.rs".to_owned()],
        hidden: true,
        limit: Some(1),
        sort: true,
        ..Options::default()
    };
    let plan = QueryPlan::compile(&options).unwrap();
    let mut runner = Runner::new(&plan, Vec::new());
    runner.run().unwrap();

    assert_eq!(runner.output, [PathBuf::from(".hidden/secret.rs")]);
    assert_eq!(runner.stats.matches, 1);
    assert!(runner.stopped);
}

#[test]
fn literal_root_preserves_type_and_hidden_semantics() {
    let fixture = Fixture::new();

    let default_dir = Options {
        cwd: fixture.0.clone(),
        positive: vec!["src".to_owned()],
        sort: true,
        ..Options::default()
    };
    let default_plan = QueryPlan::compile(&default_dir).unwrap();
    let mut default_runner = Runner::new(&default_plan, Vec::new());
    default_runner.run().unwrap();
    assert!(default_runner.output.is_empty());

    let explicit_dir = Options {
        wanted_type: WantedType::Dir,
        ..default_dir
    };
    let explicit_plan = QueryPlan::compile(&explicit_dir).unwrap();
    let mut explicit_runner = Runner::new(&explicit_plan, Vec::new());
    explicit_runner.run().unwrap();
    assert_eq!(explicit_runner.output, [PathBuf::from("src")]);

    let hidden = Options {
        cwd: fixture.0.clone(),
        positive: vec![".hidden/**/*.rs".to_owned()],
        sort: true,
        ..Options::default()
    };
    let hidden_plan = QueryPlan::compile(&hidden).unwrap();
    let mut hidden_runner = Runner::new(&hidden_plan, Vec::new());
    hidden_runner.run().unwrap();
    assert!(hidden_runner.output.is_empty());
    assert_eq!(hidden_runner.stats.dirs_opened, 0);
}

#[test]
fn simple_search_treats_glob_metacharacters_literally() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("src/config[1].rs"), b"").unwrap();
    let options = Options {
        cwd: fixture.0.clone(),
        simple_terms: vec![b"config[1]".to_vec()],
        sort: true,
        ..Options::default()
    };
    let plan = QueryPlan::compile(&options).unwrap();
    let mut runner = Runner::new(&plan, Vec::new());
    runner.run().unwrap();

    assert_eq!(runner.output, [PathBuf::from("src/config[1].rs")]);
}

#[test]
fn hierarchical_gitignore_uses_child_precedence_and_reinclusion() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("src/config[1].rs"), b"").unwrap();
    fs::write(
        fixture.0.join(".gitignore"),
        "target/\n*.toml\n!src/nested/config.toml\n",
    )
    .unwrap();
    fs::write(fixture.0.join("src/.gitignore"), "lib.rs\n").unwrap();
    let options = Options {
        cwd: fixture.0.clone(),
        positive: vec!["**/*".to_owned()],
        gitignore: true,
        sort: true,
        ..Options::default()
    };
    let plan = QueryPlan::compile(&options).unwrap();
    let mut runner = Runner::new(&plan, Vec::new());
    runner.run().unwrap();

    assert_eq!(
        runner.output,
        [
            PathBuf::from("src/config[1].rs"),
            PathBuf::from("src/nested/config.toml")
        ]
    );
    assert_eq!(runner.stats.matches, 2);
}

#[test]
fn json_escaping_produces_valid_string_content() {
    assert_eq!(
        json_escape("quote\" slash\\ newline\n"),
        "quote\\\" slash\\\\ newline\\n"
    );
}

#[test]
fn exec_command_parser_honors_quotes_and_placeholders() {
    assert_eq!(
        parse_command_line("tool --name 'hello world' {}").unwrap(),
        ["tool", "--name", "hello world", "{}"]
    );
    assert!(parse_command_line("tool 'unterminated").is_err());
}

#[test]
fn count_mode_honors_limit_without_collecting_paths() {
    let fixture = Fixture::new();
    let options = Options {
        cwd: fixture.0.clone(),
        positive: vec!["**/*.rs".to_owned()],
        count: true,
        limit: Some(1),
        ..Options::default()
    };
    let plan = QueryPlan::compile(&options).unwrap();
    let mut runner = Runner::new(&plan, Vec::new());
    runner.run().unwrap();

    assert_eq!(runner.stats.matches, 1);
    assert!(runner.output.is_empty());
    assert!(runner.stopped);
}

struct BrokenWriter;

impl Write for BrokenWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn streaming_propagates_broken_pipe_without_panicking() {
    let fixture = Fixture::new();
    let options = Options {
        cwd: fixture.0.clone(),
        positive: vec!["**/*.rs".to_owned()],
        ..Options::default()
    };
    let plan = QueryPlan::compile(&options).unwrap();
    let mut runner = Runner::new(&plan, BrokenWriter);

    let error = runner.run().unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}

#[test]
fn traverses_deep_trees_without_losing_matches() {
    let fixture = Fixture::new();
    let mut directory = fixture.0.clone();
    let mut relative = PathBuf::new();
    for _ in 0..64 {
        directory.push("d");
        relative.push("d");
    }
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("leaf.rs"), b"").unwrap();
    relative.push("leaf.rs");

    let options = Options {
        cwd: fixture.0.clone(),
        positive: vec!["**/leaf.rs".to_owned()],
        sort: true,
        ..Options::default()
    };
    let plan = QueryPlan::compile(&options).unwrap();
    let mut runner = Runner::new(&plan, Vec::new());
    runner.run().unwrap();

    assert_eq!(runner.output, [relative]);
}

#[cfg(unix)]
#[test]
fn reports_symlinks_without_following_directory_links() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    symlink(fixture.0.join("src/lib.rs"), fixture.0.join("lib-link")).unwrap();
    symlink(fixture.0.join("src"), fixture.0.join("src-link")).unwrap();

    let links = Options {
        cwd: fixture.0.clone(),
        positive: vec!["**/*".to_owned()],
        wanted_type: WantedType::Symlink,
        sort: true,
        ..Options::default()
    };
    let link_plan = QueryPlan::compile(&links).unwrap();
    let mut link_runner = Runner::new(&link_plan, Vec::new());
    link_runner.run().unwrap();
    assert_eq!(
        link_runner.output,
        [PathBuf::from("lib-link"), PathBuf::from("src-link")]
    );

    let files = Options {
        wanted_type: WantedType::File,
        ..links
    };
    let file_plan = QueryPlan::compile(&files).unwrap();
    let mut file_runner = Runner::new(&file_plan, Vec::new());
    file_runner.run().unwrap();
    assert!(
        file_runner
            .output
            .iter()
            .all(|path| !path.starts_with("src-link"))
    );
}

#[cfg(unix)]
#[test]
fn preserves_non_utf8_unix_paths() {
    use std::os::unix::ffi::OsStringExt;

    let fixture = Fixture::new();
    let name = OsString::from_vec(b"invalid-\xff.rs".to_vec());
    if !native_fixture::write(fixture.0.join(&name), b"") {
        return;
    }
    let options = Options {
        cwd: fixture.0.clone(),
        positive: vec!["**/*.rs".to_owned()],
        sort: true,
        ..Options::default()
    };
    let plan = QueryPlan::compile(&options).unwrap();
    let mut runner = Runner::new(&plan, Vec::new());
    runner.run().unwrap();

    assert!(runner.output.contains(&PathBuf::from(name)));
}

#[test]
fn parallel_traversal_matches_sequential_results() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join(".gitignore"), "target/\n").unwrap();
    let options = Options {
        cwd: fixture.0.clone(),
        positive: vec!["**/*.{rs,toml}".to_owned()],
        negative: vec!["**/target/**".to_owned()],
        gitignore: true,
        sort: true,
        ..Options::default()
    };
    let plan = QueryPlan::compile(&options).unwrap();
    let mut sequential = Runner::new(&plan, Vec::new());
    sequential.run().unwrap();
    let (stats, mut parallel) = run_parallel(&plan, 2, false).unwrap();
    parallel.sort_unstable();

    assert_eq!(parallel, sequential.output);
    assert_eq!(stats.matches as usize, parallel.len());
}
#[cfg(unix)]
#[path = "../tests/support/native_fixture.rs"]
mod native_fixture;

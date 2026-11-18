use super::{Pattern, PatternProgram, SegmentMatcher};
use std::ffi::OsStr;

fn matcher(raw: &str) -> SegmentMatcher {
    SegmentMatcher::compile(super::parse::parse_segment(raw, raw).unwrap())
}

#[test]
fn insensitive_fast_paths_match_ascii_without_changing_sensitive_mode() {
    for (pattern, filename) in [
        ("Config.toml", "cONFIG.TOML"),
        ("src*", "SRC-main"),
        ("*Readme", "projectREADME"),
    ] {
        let matcher = matcher(pattern);
        assert!(
            matcher.matches_with_case(OsStr::new(filename), true),
            "{pattern}"
        );
        assert!(!matcher.matches(OsStr::new(filename)), "{pattern}");
    }
}

#[test]
fn insensitive_general_matcher_handles_literals_and_classes() {
    for (pattern, filename) in [
        ("File?.[A-Z]s", "fILE1.rs"),
        ("[A-Z]bc", "aBC"),
        ("[a-z][0-9]", "Q7"),
        ("[!-?]x", "!X"),
        ("[!-A]", "b"),
    ] {
        assert!(
            matcher(pattern).matches_with_case(OsStr::new(filename), true),
            "{pattern} should match {filename}"
        );
    }
    assert!(!matcher("File?.[A-Z]s").matches_with_case(OsStr::new("file1.7s"), true));
    assert!(!matcher("[!-A]").matches_with_case(OsStr::new("a"), true));
}

#[test]
fn insensitive_class_negation_is_applied_after_ascii_case_variants() {
    assert!(!matcher("[!a-z]").matches_with_case(OsStr::new("Q"), true));
    assert!(matcher("[!a-z]").matches_with_case(OsStr::new("7"), true));
    assert!(!matcher("[!A-Z]").matches_with_case(OsStr::new("m"), true));
    assert!(matcher("[!A-Z]").matches_with_case(OsStr::new("-"), true));
}

#[cfg(unix)]
#[test]
fn insensitive_matching_keeps_non_ascii_bytes_exact() {
    use std::os::unix::ffi::OsStrExt;

    let upper = OsStr::from_bytes(b"name-\xC3\x84");
    let lower = OsStr::from_bytes(b"NAME-\xC3\xA4");
    assert!(matcher("name-Ä").matches_with_case(upper, true));
    assert!(!matcher("name-Ä").matches_with_case(lower, true));
}

#[test]
fn insensitive_program_advance_preserves_shared_states_and_globstar() {
    let patterns = ["src/**/File?.rs", "src/**/FILE?.rs", "src/**/mod.rs"]
        .into_iter()
        .map(|pattern| Pattern::compile(pattern.to_owned()).unwrap())
        .collect::<Vec<_>>();
    let program = PatternProgram::compile(&patterns);
    let mut states = program.initial_states();
    states = program.advance_with_case(&states, OsStr::new("SRC"), true);
    states = program.advance_with_case(&states, OsStr::new("nested"), true);
    states = program.advance_with_case(&states, OsStr::new("file1.RS"), true);
    assert!(program.states_match(&states));

    // The case-sensitive API remains useful for the ignore engine and existing callers.
    let mut sensitive = program.initial_states();
    for component in ["SRC", "nested", "file1.RS"] {
        sensitive = program.advance(&sensitive, OsStr::new(component));
    }
    assert!(!program.states_match(&sensitive));
}

#[test]
fn escaped_compile_keeps_glob_syntax_and_quotes_literal_metacharacters() {
    let pattern = Pattern::compile_escaped(r"data/\[literal\]/*.rs".to_owned()).unwrap();
    let program = PatternProgram::compile(&[pattern]);
    let mut scratch = super::PathMatchScratch::default();
    assert!(
        program
            .matches_path_with_scratch(std::path::Path::new("data/[literal]/one.rs"), &mut scratch)
    );
    assert!(
        !program
            .matches_path_with_scratch(std::path::Path::new("data/xliteralx/one.rs"), &mut scratch)
    );
    assert!(
        !program.matches_path_with_scratch(
            std::path::Path::new("data/[literal]/one.txt"),
            &mut scratch
        )
    );
}

#[test]
fn escaped_compiler_preserves_default_query_separator_behavior() {
    let default = Pattern::compile(r"data\*.rs".to_owned()).unwrap();
    assert_eq!(default.literal_prefix, [b"data".to_vec()]);
    let escaped = Pattern::compile_escaped(r"data\*.rs".to_owned()).unwrap();
    assert_eq!(escaped.literal_prefix, [r"data*.rs".as_bytes().to_vec()]);
}

#[test]
fn brace_expansion_respects_escapes_only_in_escaped_mode() {
    assert!(super::expand_braces(r"literal\{name\}.rs").is_err());
    assert_eq!(
        super::expand_braces_escaped(r"literal\{name\}.rs").unwrap(),
        [r"literal\{name\}.rs"]
    );
    assert_eq!(
        super::expand_braces_escaped(r"src/*.{rs,ts}").unwrap(),
        ["src/*.rs", "src/*.ts"]
    );
    assert_eq!(
        super::expand_braces_escaped(r"src/[{]/*.rs").unwrap(),
        [r"src/[{]/*.rs"]
    );
}

#[test]
fn malformed_ignore_class_does_not_match_empty_component() {
    let matcher = SegmentMatcher::compile(super::parse::parse_ignore_segment("[broken", "[broken"));
    assert!(!matcher.matches(OsStr::new("")));
    assert!(!matcher.matches(OsStr::new("broken")));
}

#[test]
fn unicode_mode_counts_scalars_and_uses_only_simple_lowercase() {
    let question = matcher("?");
    assert!(!question.matches_with_mode(OsStr::new("é"), false, false));
    assert!(question.matches_with_mode(OsStr::new("é"), false, true));
    assert!(matcher("Ä").matches_with_mode(OsStr::new("ä"), true, true));
    assert!(!matcher("ß").matches_with_mode(OsStr::new("SS"), true, true));
}

#[test]
fn unicode_classes_match_scalars_negation_and_ascii_insensitive_ranges() {
    fn unicode_match(pattern: &str, candidate: &str, insensitive: bool) -> bool {
        let pattern = Pattern::compile_unicode(pattern.to_owned(), false).unwrap();
        let Segment::Match(matcher) = &pattern.segments[0] else {
            panic!("expected matcher segment");
        };
        matcher.matches_with_mode(OsStr::new(candidate), insensitive, true)
    }
    assert!(unicode_match("[é]", "é", false));
    assert!(!unicode_match("[é]", "e", false));
    assert!(unicode_match("[α-ω]", "λ", false));
    assert!(!unicode_match("[α-ω]", "Ω", false));
    assert!(!unicode_match("[!α-ω]", "λ", false));
    assert!(unicode_match("[!α-ω]", "z", false));
    assert!(unicode_match("[a-z]", "A", true));
    assert!(!unicode_match("[a-z]", "A", false));
}

#[test]
fn unicode_posix_classes_keep_ascii_membership() {
    let pattern = Pattern::compile_unicode("[[:alpha:]]".to_owned(), false).unwrap();
    let Segment::Match(matcher) = &pattern.segments[0] else {
        panic!("expected matcher segment");
    };
    assert!(matcher.matches_with_mode(OsStr::new("A"), false, true));
    assert!(!matcher.matches_with_mode(OsStr::new("α"), false, true));
}

#[test]
fn unicode_classes_work_inside_extglobs() {
    let pattern = Pattern::compile_unicode("@([α-ω]|Ä)".to_owned(), false).unwrap();
    let Segment::Match(SegmentMatcher::ExtGlob(matcher)) = &pattern.segments[0] else {
        panic!("expected extglob matcher");
    };
    assert!(matcher.matches_with_mode(OsStr::new("λ"), false, true));
    assert!(matcher.matches_with_mode(OsStr::new("ä"), true, true));
    assert!(!matcher.matches_with_mode(OsStr::new("Ω"), false, true));
}

#[test]
fn path_program_passes_unicode_mode_through_each_component() {
    let pattern = Pattern::compile("src/?/Ä?.rs".to_owned()).unwrap();
    let program = PatternProgram::compile(&[pattern]);
    let mut scratch = super::PathMatchScratch::default();
    let path = std::path::Path::new("src/é/äß.rs");
    assert!(program.matches_path_with_mode_and_scratch(path, true, true, &mut scratch));
    assert!(!program.matches_path_with_mode_and_scratch(path, true, false, &mut scratch));
}

#[cfg(unix)]
#[test]
fn unicode_mode_falls_back_for_invalid_native_bytes_and_ignore_patterns_retain_them() {
    use std::os::unix::ffi::OsStrExt;
    assert!(matcher("?").matches_with_mode(OsStr::from_bytes(b"\xff"), false, true));
    let pattern = Pattern::compile_ignore_bytes(b"bad\xff*".to_vec()).unwrap();
    let super::Segment::Match(matcher) = &pattern.segments[0] else {
        panic!("expected matcher segment");
    };
    assert!(matcher.matches(OsStr::from_bytes(b"bad\xffname")));
    assert!(!matcher.matches(OsStr::from_bytes(b"bad\xfename")));
}
use super::Segment;

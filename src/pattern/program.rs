use super::{Pattern, Segment};
use std::ffi::OsStr;
use std::path::{Component, Path};
#[derive(Clone, Debug)]
pub(crate) struct ProgramNode {
    pub(crate) edges: Vec<(Segment, usize)>,
    pub(crate) globstar: bool,
    pub(crate) terminal: bool,
    pub(crate) subtree_terminal: bool,
}

impl ProgramNode {
    pub(crate) fn new(globstar: bool) -> Self {
        Self {
            edges: Vec::new(),
            globstar,
            terminal: false,
            subtree_terminal: false,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PatternProgram {
    pub(crate) nodes: Vec<ProgramNode>,
}

#[derive(Default)]
pub(crate) struct PathMatchScratch {
    states: Vec<usize>,
    next: Vec<usize>,
}

impl PatternProgram {
    pub(crate) fn compile(patterns: &[Pattern]) -> Self {
        let mut nodes = vec![ProgramNode::new(false)];
        for pattern in patterns {
            let mut node = 0;
            for segment in &pattern.segments {
                let existing = nodes[node]
                    .edges
                    .iter()
                    .find_map(|(edge, child)| (edge == segment).then_some(*child));
                node = if let Some(child) = existing {
                    child
                } else {
                    let child = nodes.len();
                    nodes.push(ProgramNode::new(matches!(segment, Segment::GlobStar)));
                    nodes[node].edges.push((segment.clone(), child));
                    child
                };
            }
            nodes[node].terminal = true;
            if matches!(pattern.segments.last(), Some(Segment::GlobStar)) {
                nodes[node].subtree_terminal = true;
            }
        }
        Self { nodes }
    }

    pub(crate) fn initial_states(&self) -> Vec<usize> {
        let mut states = Vec::new();
        self.initial_states_into(&mut states);
        states
    }

    fn initial_states_into(&self, states: &mut Vec<usize>) {
        states.clear();
        if self.nodes.len() == 1
            && !self.nodes[0].terminal
            && self.nodes[0].edges.is_empty()
            && !self.nodes[0].globstar
        {
            return;
        }
        states.push(0);
        self.add_epsilon_closure(states);
    }

    #[cfg(test)]
    pub(crate) fn advance(&self, states: &[usize], component: &OsStr) -> Vec<usize> {
        self.advance_with_case(states, component, false)
    }

    #[cfg(test)]
    pub(crate) fn advance_with_case(
        &self,
        states: &[usize],
        component: &OsStr,
        insensitive: bool,
    ) -> Vec<usize> {
        self.advance_with_mode(states, component, insensitive, false)
    }

    pub(crate) fn advance_with_mode(
        &self,
        states: &[usize],
        component: &OsStr,
        insensitive: bool,
        unicode: bool,
    ) -> Vec<usize> {
        let mut next = Vec::with_capacity(states.len());
        self.advance_into_with_mode(states, component, insensitive, unicode, &mut next);
        next
    }

    /// Reuse entry-local storage without modifying the parent's active states.
    #[cfg(test)]
    pub(crate) fn advance_into_with_case(
        &self,
        states: &[usize],
        component: &OsStr,
        insensitive: bool,
        next: &mut Vec<usize>,
    ) {
        self.advance_into_with_mode(states, component, insensitive, false, next)
    }

    pub(crate) fn advance_into_with_mode(
        &self,
        states: &[usize],
        component: &OsStr,
        insensitive: bool,
        unicode: bool,
        next: &mut Vec<usize>,
    ) {
        next.clear();
        for &node in states {
            if self.nodes[node].globstar {
                push_unique(next, node);
            }
            for (segment, child) in &self.nodes[node].edges {
                match segment {
                    Segment::GlobStar => push_unique(next, *child),
                    Segment::Match(matcher)
                        if matcher.matches_with_mode(component, insensitive, unicode) =>
                    {
                        push_unique(next, *child);
                    }
                    Segment::Match(_) => {}
                }
            }
        }
        self.add_epsilon_closure(next);
    }

    pub(crate) fn matches_path_with_scratch(
        &self,
        path: &Path,
        scratch: &mut PathMatchScratch,
    ) -> bool {
        self.matches_path_with_case_and_scratch(path, false, scratch)
    }

    pub(crate) fn matches_path_with_case_and_scratch(
        &self,
        path: &Path,
        insensitive: bool,
        scratch: &mut PathMatchScratch,
    ) -> bool {
        self.matches_path_with_mode_and_scratch(path, insensitive, false, scratch)
    }

    pub(crate) fn matches_path_with_mode_and_scratch(
        &self,
        path: &Path,
        insensitive: bool,
        unicode: bool,
        scratch: &mut PathMatchScratch,
    ) -> bool {
        self.initial_states_into(&mut scratch.states);
        if scratch.states.is_empty() {
            return false;
        }
        for component in path.components().filter_map(|component| match component {
            Component::Normal(component) => Some(component),
            _ => None,
        }) {
            self.advance_into_with_mode(
                &scratch.states,
                component,
                insensitive,
                unicode,
                &mut scratch.next,
            );
            std::mem::swap(&mut scratch.states, &mut scratch.next);
        }
        self.states_match(&scratch.states)
    }

    pub(crate) fn states_match(&self, states: &[usize]) -> bool {
        states.iter().any(|node| self.nodes[*node].terminal)
    }

    pub(crate) fn states_descendant_possible(&self, states: &[usize]) -> bool {
        states
            .iter()
            .any(|node| self.nodes[*node].globstar || !self.nodes[*node].edges.is_empty())
    }

    pub(crate) fn states_exclude_subtree(&self, states: &[usize]) -> bool {
        states.iter().any(|node| self.nodes[*node].subtree_terminal)
    }

    pub(crate) fn add_epsilon_closure(&self, states: &mut Vec<usize>) {
        let mut cursor = 0;
        while cursor < states.len() {
            let node = states[cursor];
            for (segment, child) in &self.nodes[node].edges {
                if matches!(segment, Segment::GlobStar) {
                    push_unique(states, *child);
                }
            }
            cursor += 1;
        }
    }
}

pub(crate) fn push_unique(states: &mut Vec<usize>, node: usize) {
    if !states.contains(&node) {
        states.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reused_entry_storage_clears_siblings_without_changing_parent_states() {
        let positives = ["src/**/keep.rs", "src/root.rs"]
            .map(|source| Pattern::compile(source.to_owned()).unwrap());
        let program = PatternProgram::compile(&positives);
        let parent = program.advance(&program.initial_states(), OsStr::new("src"));
        let original_parent = parent.clone();
        let mut scratch = vec![usize::MAX];
        for (name, selected) in [
            ("keep.rs", true),
            ("missing.rs", false),
            ("root.rs", true),
            ("KEEP.RS", false),
            ("another.rs", false),
        ] {
            program.advance_into_with_case(&parent, OsStr::new(name), false, &mut scratch);
            assert_eq!(program.states_match(&scratch), selected, "{name}");
            assert_eq!(parent, original_parent);
        }
        program.advance_into_with_case(&parent, OsStr::new("KEEP.RS"), true, &mut scratch);
        assert!(program.states_match(&scratch));
        let empty = PatternProgram::compile(&[]);
        empty.advance_into_with_case(&[], OsStr::new("keep.rs"), false, &mut scratch);
        assert!(scratch.is_empty());
    }

    #[test]
    fn reusable_path_scratch_does_not_leak_between_candidates() {
        let patterns = ["**/keep.rs", "**/nested/*.toml"]
            .map(|source| Pattern::compile(source.to_owned()).unwrap());
        let program = PatternProgram::compile(&patterns);
        let mut scratch = PathMatchScratch::default();
        for (path, expected) in [
            (Path::new("one/keep.rs"), true),
            (Path::new("one/drop.rs"), false),
            (Path::new("nested/Cargo.toml"), true),
            (Path::new("nested/deep/keep.rs"), true),
            (Path::new("nested/deep/drop.rs"), false),
        ] {
            assert_eq!(
                program.matches_path_with_scratch(path, &mut scratch),
                expected,
                "{}",
                path.display()
            );
        }
    }
}

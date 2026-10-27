use super::{Pattern, Segment};
use std::ffi::OsStr;
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
        let mut states = vec![0];
        self.add_epsilon_closure(&mut states);
        states
    }

    pub(crate) fn advance(&self, states: &[usize], component: &OsStr) -> Vec<usize> {
        let mut next = Vec::with_capacity(states.len());
        for &node in states {
            if self.nodes[node].globstar {
                push_unique(&mut next, node);
            }
            for (segment, child) in &self.nodes[node].edges {
                match segment {
                    Segment::GlobStar => push_unique(&mut next, *child),
                    Segment::Match(matcher) if matcher.matches(component) => {
                        push_unique(&mut next, *child);
                    }
                    Segment::Match(_) => {}
                }
            }
        }
        self.add_epsilon_closure(&mut next);
        next
    }

    pub(crate) fn matches_path(&self, components: &[&OsStr]) -> bool {
        let states = components
            .iter()
            .fold(self.initial_states(), |states, component| {
                self.advance(&states, component)
            });
        self.states_match(&states)
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

//! Offline path denotation on the scraper facade.
//!
//! From a source node, yields destinations ordered by axis rank:
//! - document order for downward / forward axes,
//! - nearest-first for upward / backward axes.
//!
//! Path union is Boolean; bag multiplicity belongs to the relational layer.

use std::collections::HashSet;

use scraper::Selector;

use super::dom::Dom;
use crate::matcher::Step;
use crate::plan::representation::id::NodeId;
use crate::plan::representation::path::{NodeTest, Path};

/// Destinations of `path` from `source`, in axis-rank order.
pub(crate) fn eval(dom: &Dom, path: &Path, source: NodeId) -> Vec<NodeId> {
    eval_dir(dom, path, source, false)
}

fn eval_dir(dom: &Dom, path: &Path, source: NodeId, converse: bool) -> Vec<NodeId> {
    match path {
        Path::Empty => Vec::new(),
        Path::Id => vec![source],
        Path::Child if !converse => children(dom, source),
        Path::Child => dom.parent(source).into_iter().collect(),
        Path::Next if !converse => dom.next_sibling(source).into_iter().collect(),
        Path::Next => dom.prev_sibling(source).into_iter().collect(),
        Path::Test(test) => {
            if node_matches(dom, source, test) {
                vec![source]
            } else {
                Vec::new()
            }
        },
        Path::Union(left, right) => merge_dedup(eval_dir(dom, left, source, converse), eval_dir(dom, right, source, converse)),
        Path::Seq(left, right) if !converse => compose(eval_dir(dom, left, source, false), |y| eval_dir(dom, right, y, false)),
        // (a;b)˘ = b˘;a˘
        Path::Seq(left, right) => compose(eval_dir(dom, right, source, true), |y| eval_dir(dom, left, y, true)),
        Path::Star(inner) => eval_star(dom, inner, source, converse),
        Path::Converse(inner) => eval_dir(dom, inner, source, !converse),
    }
}

/// `r*`. Child\* uses DFS so descendants come in document order; other axes use path-length order
/// (nearest-first for parent / prev).
fn eval_star(dom: &Dom, inner: &Path, source: NodeId, converse: bool) -> Vec<NodeId> {
    if !converse && matches!(inner, Path::Child) {
        return child_star_doc_order(dom, source);
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    seen.insert(source);
    out.push(source);
    let mut i = 0;
    while i < out.len() {
        for dest in eval_dir(dom, inner, out[i], converse) {
            if seen.insert(dest) {
                out.push(dest);
            }
        }
        i += 1;
    }
    out
}

fn child_star_doc_order(dom: &Dom, source: NodeId) -> Vec<NodeId> {
    let mut out = vec![source];
    push_descendants_dfs(dom, source, &mut out);
    out
}

fn push_descendants_dfs(dom: &Dom, id: NodeId, out: &mut Vec<NodeId>) {
    let mut child = dom.first_child(id);
    while let Some(ch) = child {
        out.push(ch);
        push_descendants_dfs(dom, ch, out);
        child = dom.next_sibling(ch);
    }
}

fn children(dom: &Dom, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut child = dom.first_child(id);
    while let Some(ch) = child {
        out.push(ch);
        child = dom.next_sibling(ch);
    }
    out
}

fn compose(sources: Vec<NodeId>, mut step: impl FnMut(NodeId) -> Vec<NodeId>) -> Vec<NodeId> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for y in sources {
        for z in step(y) {
            if seen.insert(z) {
                out.push(z);
            }
        }
    }
    out
}

fn merge_dedup(left: Vec<NodeId>, right: Vec<NodeId>) -> Vec<NodeId> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for id in left.into_iter().chain(right) {
        if seen.insert(id) {
            out.push(id);
        }
    }
    out
}

fn node_matches(dom: &Dom, id: NodeId, test: &NodeTest) -> bool {
    match test {
        NodeTest::False => false,
        NodeTest::True => true,
        NodeTest::Not(inner) => !node_matches(dom, id, inner),
        NodeTest::Or(a, b) => node_matches(dom, id, a) || node_matches(dom, id, b),
        NodeTest::And(a, b) => node_matches(dom, id, a) && node_matches(dom, id, b),
        NodeTest::Match(pattern) => css_only_matches(dom, id, pattern.steps()),
    }
}

/// Only a single CSS [`Step::Filter`] with no predicate. Complex MatchPattern support is postponed.
fn css_only_matches(dom: &Dom, id: NodeId, steps: &[Step]) -> bool {
    assert!(steps.len() == 1, "offline path NodeTest::Match supports one CSS Filter only (full MatchPattern support is postponed)");
    match &steps[0] {
        Step::Filter(css, None) => {
            let sel = Selector::parse(css).unwrap_or_else(|e| panic!("invalid CSS `{css}`: {e:?}"));
            dom.matches(id, &sel)
        },
        Step::Filter(_, Some(_)) => panic!("offline path NodeTest::Match does not support predicates yet (full MatchPattern support is postponed)"),
        _ => panic!("offline path NodeTest::Match supports CSS Filter only (full MatchPattern support is postponed)"),
    }
}

#[cfg(test)]
#[path = "path.test.rs"]
mod test;

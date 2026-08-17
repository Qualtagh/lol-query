//! Lower Expand along `C` / `C+` to an Engine chain + parent-gated birth.

use std::rc::Rc;

use crate::engine::{ChainId, Engine};
use crate::matcher::Step;
use crate::plan::physical::down::DownExtent;
use crate::plan::representation::path::{NodeTest, Path};

/// Recognized downward expand: extent for `down_ok` + Engine steps for the leaf test.
pub(crate) struct DownExpand {
    pub extent: DownExtent,
    pub steps: Vec<Step>,
}

/// Classify `C;[predicate]`, `C+;[predicate]`, or the unfiltered forms (predicate = true).
pub(crate) fn classify_down_path(path: &Path) -> Option<DownExtent> {
    match path {
        Path::Seq(left, right) if matches!(right.as_ref(), Path::Test(_)) => classify_down_axis(left),
        other => classify_down_axis(other),
    }
}

/// Consume a downward path into extent + Engine steps (moves the matcher; no step clone).
pub(crate) fn take_down_expand(path: Path) -> Option<DownExpand> {
    let (extent, test) = match path {
        Path::Seq(left, right) => match *right {
            Path::Test(test) => (classify_down_axis(&left)?, test),
            other => {
                let path = Path::Seq(left, Box::new(other));
                (classify_down_axis(&path)?, NodeTest::True)
            },
        },
        other => (classify_down_axis(&other)?, NodeTest::True),
    };
    Some(DownExpand { extent, steps: node_test_into_steps(test) })
}

fn classify_down_axis(path: &Path) -> Option<DownExtent> {
    match path {
        Path::Child => Some(DownExtent::Child),
        // C+ = C ; C*
        Path::Seq(left, right)
            if matches!(left.as_ref(), Path::Child) && matches!(right.as_ref(), Path::Star(inner) if matches!(inner.as_ref(), Path::Child)) =>
        {
            Some(DownExtent::Descendant)
        },
        _ => None,
    }
}

fn node_test_into_steps(test: NodeTest) -> Vec<Step> {
    match test {
        NodeTest::True => vec![Step::Filter("*".into(), None)],
        NodeTest::False => panic!("down lower: False node test matches nothing; refuse Engine chain"),
        NodeTest::Match(pattern) => {
            Rc::try_unwrap(pattern).unwrap_or_else(|_| panic!("down lower: MatchPattern still shared; expected unique after take")).into_steps()
        },
        NodeTest::Not(_) | NodeTest::Or(_, _) | NodeTest::And(_, _) => {
            panic!("down lower: boolean NodeTest combinators not lowered yet")
        },
    }
}

/// Add a chain for `expand` and return its id.
pub(crate) fn bind_chain(engine: &mut Engine, expand: DownExpand) -> ChainId {
    engine.add_chain(expand.steps)
}

#[cfg(test)]
#[path = "bind_down.test.rs"]
mod test;

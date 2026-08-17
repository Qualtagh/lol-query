use super::{classify_down_path, take_down_expand};
use crate::matcher::MatchPattern;
use crate::matcher::Step;
use crate::plan::physical::down::DownExtent;
use crate::plan::representation::path::Path;

#[test]
fn classify_find_and_children() {
    fn test_path(path: &Path, expected: Option<DownExtent>) {
        assert_eq!(classify_down_path(path), expected);
    }
    test_path(&Path::find(MatchPattern::new().css(".item")), Some(DownExtent::Descendant));
    test_path(&Path::children(MatchPattern::new().css("div")), Some(DownExtent::Child));
    test_path(&Path::child_axis().plus(), Some(DownExtent::Descendant));
    test_path(&Path::child_axis(), Some(DownExtent::Child));
    test_path(&Path::next(), None);
}

#[test]
fn take_moves_css_steps() {
    let down = take_down_expand(Path::find(MatchPattern::new().css(".item"))).unwrap();
    assert_eq!(down.extent, DownExtent::Descendant);
    assert!(matches!(&down.steps[..], [Step::Filter(sel, None)] if sel == ".item"));
}

use crate::plan::offline::dom::Dom;
use crate::plan::representation::id::NodeId;
use crate::plan::representation::registry::Value;

/// `@root` -> synthetic root; else first element with that `id`, else that tag.
pub(crate) fn at(dom: &Dom, key: &str) -> NodeId {
    if key == "@root" {
        return dom.root();
    }

    let mut stack = vec![dom.root()];
    let mut by_tag = None;
    while let Some(id) = stack.pop() {
        if dom.attr(id, "id") == Some(key) {
            return id;
        }
        if by_tag.is_none() && dom.tag(id) == Some(key) {
            by_tag = Some(id);
        }
        let mut child = dom.first_child(id);
        let mut kids = Vec::new();
        while let Some(c) = child {
            kids.push(c);
            child = dom.next_sibling(c);
        }
        stack.extend(kids.into_iter().rev());
    }

    by_tag.unwrap_or_else(|| panic!("no node for `{key}`"))
}

/// Caller-side `unwrap_or_default` for missing attrs (`Null` -> `""`).
// TODO: consider introducing map_single or another simplified version of map
pub(crate) fn unwrap_or_empty_str(args: &[Value]) -> Value {
    assert!(args.len() == 1, "unwrap_or_empty_str expects one argument");
    match &args[0] {
        Value::Null => Value::str(""),
        other => other.clone(),
    }
}

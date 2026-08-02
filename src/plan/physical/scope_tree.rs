/// Depth of `node` in the forest: number of parent hops to a root.
pub(crate) fn depth(parent: &[Option<usize>], mut node: usize) -> u32 {
    let mut d = 0;
    while let Some(p) = parent[node] {
        d += 1;
        node = p;
    }
    d
}

/// True when `ancestor == descendant`, or `ancestor` lies on the path from `descendant` to its root.
pub(crate) fn is_ancestor_or_eq(parent: &[Option<usize>], ancestor: usize, mut descendant: usize) -> bool {
    if ancestor == descendant {
        return true;
    }
    while let Some(p) = parent[descendant] {
        if p == ancestor {
            return true;
        }
        descendant = p;
    }
    false
}

/// Strict: `descendant` is properly under `ancestor` (not equal).
pub(crate) fn is_under(parent: &[Option<usize>], ancestor: usize, descendant: usize) -> bool {
    ancestor != descendant && is_ancestor_or_eq(parent, ancestor, descendant)
}

/// Nearest common ancestor of `a` and `b` (either node if one is under the other; either if equal).
pub(crate) fn nearest_common_ancestor(parent: &[Option<usize>], mut a: usize, mut b: usize) -> usize {
    let mut da = depth(parent, a);
    let mut db = depth(parent, b);
    while da > db {
        a = parent[a].unwrap();
        da -= 1;
    }
    while db > da {
        b = parent[b].unwrap();
        db -= 1;
    }
    while a != b {
        a = parent[a].unwrap();
        b = parent[b].unwrap();
    }
    a
}

#[cfg(test)]
#[path = "scope_tree.test.rs"]
mod test;

/// How far a downward axis may reach from an open parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownExtent {
    /// Immediate children only: `child_depth == parent_depth + 1`.
    Child,
    /// Proper descendants: `child_depth > parent_depth`.
    Descendant,
}

/// Whether a matched element at `child_depth` lies on this down-move from an open parent at `parent_depth`.
pub(crate) fn down_ok(extent: DownExtent, parent_depth: u32, child_depth: u32) -> bool {
    match extent {
        DownExtent::Child => child_depth == parent_depth + 1,
        DownExtent::Descendant => child_depth > parent_depth,
    }
}

#[cfg(test)]
#[path = "down.test.rs"]
mod test;

use super::{depth, is_ancestor_or_eq, is_under, nearest_common_ancestor};

/// Forest:
/// ```text
///   0
///   ├── 1
///   │   ├── 3
///   │   └── 4
///   └── 2
/// ```
fn parents() -> [Option<usize>; 5] {
    [None, Some(0), Some(0), Some(1), Some(1)]
}

#[test]
fn ancestry_cases() {
    let p = parents();

    assert_eq!(depth(&p, 0), 0);
    assert_eq!(depth(&p, 1), 1);
    assert_eq!(depth(&p, 3), 2);

    assert!(is_ancestor_or_eq(&p, 0, 0));
    assert!(is_ancestor_or_eq(&p, 0, 3));
    assert!(is_ancestor_or_eq(&p, 1, 4));
    assert!(!is_ancestor_or_eq(&p, 2, 3));
    assert!(!is_ancestor_or_eq(&p, 3, 1));

    assert!(!is_under(&p, 0, 0));
    assert!(is_under(&p, 0, 3));
    assert!(!is_under(&p, 2, 3));

    assert_eq!(nearest_common_ancestor(&p, 3, 4), 1);
    assert_eq!(nearest_common_ancestor(&p, 3, 2), 0);
    assert_eq!(nearest_common_ancestor(&p, 1, 3), 1);
    assert_eq!(nearest_common_ancestor(&p, 4, 4), 4);
}

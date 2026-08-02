use super::{DownExtent, down_ok};

#[test]
fn down_ok_cases() {
    // Child: exactly one deeper
    assert!(down_ok(DownExtent::Child, 0, 1));
    assert!(down_ok(DownExtent::Child, 3, 4));
    assert!(!down_ok(DownExtent::Child, 0, 0));
    assert!(!down_ok(DownExtent::Child, 0, 2));
    assert!(!down_ok(DownExtent::Child, 2, 1));

    // Descendant: any strictly deeper
    assert!(down_ok(DownExtent::Descendant, 0, 1));
    assert!(down_ok(DownExtent::Descendant, 0, 5));
    assert!(down_ok(DownExtent::Descendant, 2, 3));
    assert!(!down_ok(DownExtent::Descendant, 0, 0));
    assert!(!down_ok(DownExtent::Descendant, 3, 3));
    assert!(!down_ok(DownExtent::Descendant, 3, 2));
}

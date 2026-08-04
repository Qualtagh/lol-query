//! Utilities: bitset generalized regular expressions (Engine substrate).

mod general_regex;

pub(crate) use general_regex::{GenRegExp, Pattern};

#[cfg(test)]
pub(crate) use general_regex::{operation_count, reset_operation_count};

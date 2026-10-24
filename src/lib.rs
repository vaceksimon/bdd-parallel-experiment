pub mod parallel;
pub mod sequential;

#[cfg(any(test, feature = "test-utils"))]
#[allow(dead_code)]
pub mod test_utils;

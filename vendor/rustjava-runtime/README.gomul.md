# GOmul dependency patch

Based on the unmodified crates.io rustjava-runtime 0.1.1 package (MIT, Inseok Lee). Local fixes preserve complete EUC-KR byte pairs across reader chunks and raise a Java NullPointerException for String(null byte array) instead of panicking in Rust.

The packaged upstream integration-test target requires its unpublished test_utils crate and is not enabled here. GOmul tests the patches using the library boundary unit test and its existing JVM test harness. Upstream integration sources are retained for reference.

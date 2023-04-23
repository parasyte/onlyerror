#[test]
fn compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("compile_tests/empty.rs");
    t.pass("compile_tests/one_comment.rs");
    t.pass("compile_tests/one_param.rs");
    t.compile_fail("compile_tests/one_non_signed.rs");
    t.pass("compile_tests/multiple_variant.rs");
    t.compile_fail("compile_tests/multiple_non_signed.rs");
    t.compile_fail("compile_tests/multiple_one_non_signed.rs");
    t.pass("compile_tests/no_display.rs");
    t.compile_fail("compile_tests/no_display_no_impl.rs");
}

#[test]
fn derive_ui() {
    let t = trybuild::TestCases::check_only();
    t.pass("ui-tests/pass.rs");
    t.compile_fail("ui-tests/fail_*.rs");
}

#[test]
fn derive_try() {
    let t = trybuild::TestCases::check_only();
    t.pass("ui-tests/try.rs");
}

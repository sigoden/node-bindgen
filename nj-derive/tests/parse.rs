#[test]
fn ui() {
    let t = trybuild::TestCases::check_only();
    t.pass("ui-tests/pass.rs");
    t.compile_fail("ui-tests/fail.rs");
}

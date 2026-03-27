#[test]
#[ignore = "Enable once UI stderr snapshots are recorded"]
fn ui_contracts() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

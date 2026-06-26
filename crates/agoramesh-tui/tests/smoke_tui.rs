//! Smoke test for the agoramesh-tui binary.

#[test]
fn scaffold_binary_smoke() {
    let mut cmd =
        assert_cmd::Command::cargo_bin("agoramesh-tui").expect("agoramesh-tui binary exists");
    cmd.arg("--help").assert().success();
}

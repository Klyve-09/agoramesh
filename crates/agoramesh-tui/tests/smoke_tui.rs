//! Smoke test for the agoramesh-tui binary.

use predicates::str::contains;

#[test]
fn scaffold_binary_smoke() {
    let mut cmd =
        assert_cmd::Command::cargo_bin("agoramesh-tui").expect("agoramesh-tui binary exists");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(contains("AgoraMesh 최소 터미널 UI"))
        .stdout(contains(
            "키, 저장소, 피어, TUI 상태를 저장할 데이터 디렉터리",
        ))
        .stdout(contains("개발용 평문 키 모드"));
}

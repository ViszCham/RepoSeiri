use super::*;
use std::io::Write;

fn helper_spec(mode: &str) -> ProcessSpec {
    ProcessSpec::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "supervisor::tests::supervisor_child",
            "--ignored",
            "--nocapture",
        ])
        .env("REPOSEIRI_SUPERVISOR_CHILD", mode)
}

#[test]
fn missing_executable_is_typed() {
    let failure = run(&ProcessSpec::new(
        "reposeiri-supervisor-intentionally-missing-executable",
    ))
    .expect_err("missing executable");
    assert_eq!(failure.kind, ProcessFailureKind::MissingExecutable);
    assert_eq!(failure.exit_code, None);
}

#[test]
fn windows_application_control_spawn_error_is_environment_blocked() {
    for code in [4551, 1260] {
        assert_eq!(
            classify_spawn_error(&io::Error::from_raw_os_error(code)),
            ProcessFailureKind::EnvironmentBlocked
        );
    }
}

#[test]
fn nonzero_exit_is_typed() {
    let failure = run(&helper_spec("nonzero")).expect_err("nonzero child");
    assert_eq!(failure.kind, ProcessFailureKind::NonZeroExit);
    assert_eq!(failure.exit_code, Some(7));
}

#[test]
fn hung_child_is_killed_and_reaped() {
    let failure = run(&helper_spec("timeout").timeout(Duration::from_millis(100)))
        .expect_err("timed out child");
    assert_eq!(failure.kind, ProcessFailureKind::TimedOut);
    assert!(failure.elapsed < Duration::from_secs(5));
}

#[test]
fn output_flood_is_bounded_and_typed() {
    let failure = run(&helper_spec("flood").output_limits(1024, 1024)).expect_err("flooding child");
    assert_eq!(failure.kind, ProcessFailureKind::OutputLimitExceeded);
    assert!(failure.stdout.len() <= 1024);
    assert!(failure.stderr.len() <= 1024);
}

#[test]
#[ignore = "spawned only by supervisor tests"]
fn supervisor_child() {
    match std::env::var("REPOSEIRI_SUPERVISOR_CHILD").as_deref() {
        Ok("timeout") => thread::sleep(Duration::from_secs(10)),
        Ok("flood") => {
            let mut stdout = io::stdout().lock();
            let block = [b'x'; 8 * 1024];
            for _ in 0..128 {
                stdout.write_all(&block).expect("write flood");
            }
            stdout.flush().expect("flush flood");
        }
        Ok("nonzero") => std::process::exit(7),
        _ => {}
    }
}

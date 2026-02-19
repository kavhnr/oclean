use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nix::errno::Errno;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

fn oclean_bin() -> &'static str {
    env!("CARGO_BIN_EXE_oclean")
}

fn unique_temp_dir(prefix: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_nanos(0))
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn write_script(path: &Path, contents: &str) -> io::Result<()> {
    fs::write(path, contents)?;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)
}

fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if path.is_file() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

fn wait_dead(pid: i32, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !pid_exists(pid) {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    !pid_exists(pid)
}

fn pid_exists(pid: i32) -> bool {
    !matches!(kill(Pid::from_raw(pid), None), Err(Errno::ESRCH))
}

#[test]
fn passthrough_version_works() {
    let output = Command::new(oclean_bin())
        .env("OCLEAN_OPENCODE", "/bin/echo")
        .arg("--version")
        .output()
        .expect("failed to execute oclean --version");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "--version");
}

#[test]
fn recursive_invocation_is_blocked() {
    let output = Command::new(oclean_bin())
        .env("OCLEAN_ACTIVE", "1")
        .arg("--version")
        .output()
        .expect("failed to execute recursive oclean");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("recursive invocation detected"));
}

#[test]
fn kills_detached_child_on_signal() {
    let temp = unique_temp_dir("oclean-signal-test").expect("failed to create temp directory");
    let script_path = temp.join("opencode-mock.sh");
    let pid_file = temp.join("child.pid");

    write_script(
        &script_path,
        "#!/bin/sh\nset -eu\npid_file=\"${OCLEAN_TEST_CHILD_PID_FILE:?}\"\nsleep 30 &\nchild=\"$!\"\nprintf '%s' \"$child\" > \"$pid_file\"\nwhile :; do sleep 1; done\n",
    )
    .expect("failed to write mock opencode script");

    let mut child = Command::new(oclean_bin())
        .env("OCLEAN_OPENCODE", &script_path)
        .env("OCLEAN_TEST_CHILD_PID_FILE", &pid_file)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to launch oclean");

    assert!(wait_for_file(&pid_file, Duration::from_secs(3)));
    let spawned_pid_text = fs::read_to_string(&pid_file).expect("failed to read child pid file");
    let spawned_pid = spawned_pid_text
        .trim()
        .parse::<i32>()
        .expect("invalid child pid value");
    assert!(pid_exists(spawned_pid));

    let parent_pid = i32::try_from(child.id()).expect("pid did not fit i32");
    kill(Pid::from_raw(parent_pid), Signal::SIGTERM).expect("failed to signal oclean");
    let status = child.wait().expect("failed to wait for oclean exit");
    assert!(!status.success());

    assert!(wait_dead(spawned_pid, Duration::from_secs(3)));

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn post_exit_sweep_kills_background_child() {
    let temp = unique_temp_dir("oclean-post-exit-test").expect("failed to create temp directory");
    let script_path = temp.join("opencode-mock.sh");
    let pid_file = temp.join("child.pid");

    write_script(
        &script_path,
        "#!/bin/sh\nset -eu\npid_file=\"${OCLEAN_TEST_CHILD_PID_FILE:?}\"\nsleep 30 &\nchild=\"$!\"\nprintf '%s' \"$child\" > \"$pid_file\"\nsleep 1\nexit 0\n",
    )
    .expect("failed to write mock script");

    let status = Command::new(oclean_bin())
        .env("OCLEAN_OPENCODE", &script_path)
        .env("OCLEAN_TEST_CHILD_PID_FILE", &pid_file)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run oclean");

    assert!(status.success());
    assert!(wait_for_file(&pid_file, Duration::from_secs(2)));

    let spawned_pid_text = fs::read_to_string(&pid_file).expect("failed to read child pid file");
    let spawned_pid = spawned_pid_text
        .trim()
        .parse::<i32>()
        .expect("invalid child pid value");

    assert!(wait_dead(spawned_pid, Duration::from_secs(3)));

    let _ = fs::remove_dir_all(temp);
}

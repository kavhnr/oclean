use std::env;
use std::ffi::OsString;
use std::io::IsTerminal;
use std::process::{Child, Command, ExitCode, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;

use crate::cleanup::{cleanup_tree, post_exit_cleanup, Tracker};
use crate::resolve::{is_passthrough_mode, resolve_opencode_executable};

const SIGNALS: [i32; 4] = [SIGHUP, SIGINT, SIGTERM, SIGQUIT];
const ACTIVE_ENV: &str = "OCLEAN_ACTIVE";
const POLL_INTERVAL: Duration = Duration::from_millis(150);
const OBSERVE_INTERVAL: Duration = Duration::from_millis(800);
const PARENT_CHECK_INTERVAL: Duration = Duration::from_millis(400);
const REAP_TIMEOUT: Duration = Duration::from_millis(1_500);

#[derive(Clone, Copy)]
enum Event {
    Signal(i32),
    ParentGone,
}

pub fn entrypoint() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("oclean: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    if env::var_os(ACTIVE_ENV).is_some() {
        return Err(
            "recursive invocation detected; set OCLEAN_OPENCODE to the real opencode binary"
                .to_owned(),
        );
    }

    let opencode_bin = resolve_opencode_executable()?;
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    debug(&format!("argv count: {}", args.len()));

    if is_passthrough_mode(&args) {
        debug("passthrough mode enabled");
        return passthrough_mode(&opencode_bin, &args);
    }

    debug("starting opencode child process");
    let mut child = Command::new(opencode_bin)
        .args(&args)
        .env(ACTIVE_ENV, "1")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to start opencode: {error}"))?;

    let root_pid = i32::try_from(child.id())
        .map_err(|_| "child PID does not fit into i32 on this platform".to_owned())?;

    let mut tracker = Tracker::new(root_pid);
    let should_watch_parent = env::var_os("OCLEAN_WATCH_PARENT").is_some()
        && (std::io::stdin().is_terminal() || std::io::stdout().is_terminal());
    let event_rx = install_event_channel(should_watch_parent)?;
    tracker.observe();
    let mut last_observed = Instant::now();

    loop {
        if last_observed.elapsed() >= OBSERVE_INTERVAL {
            tracker.observe();
            last_observed = Instant::now();
        }

        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed while waiting for opencode: {error}"))?
        {
            debug("opencode child exited; running post-exit sweep");
            post_exit_cleanup(&mut tracker);
            return Ok(exit_code_from_status(status));
        }

        match event_rx.recv_timeout(POLL_INTERVAL) {
            Ok(Event::Signal(raw_signal)) => {
                debug(&format!("received signal {raw_signal}; cleaning up"));
                cleanup_tree(&mut tracker);
                reap_child(&mut child);
                return Ok(exit_code_from_signal(raw_signal));
            }
            Ok(Event::ParentGone) => {
                debug("detected parent death; cleaning up as SIGHUP");
                cleanup_tree(&mut tracker);
                reap_child(&mut child);
                return Ok(exit_code_from_signal(SIGHUP));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err("signal handler thread stopped unexpectedly".to_owned());
            }
        }
    }
}

fn passthrough_mode(
    opencode_bin: &std::path::PathBuf,
    args: &[OsString],
) -> Result<ExitCode, String> {
    debug("running passthrough child");
    let status = Command::new(opencode_bin)
        .args(args)
        .env(ACTIVE_ENV, "1")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("failed to run opencode passthrough: {error}"))?;
    Ok(exit_code_from_status(status))
}

fn install_event_channel(watch_parent: bool) -> Result<Receiver<Event>, String> {
    let mut signals = Signals::new(SIGNALS)
        .map_err(|error| format!("failed to register signal handlers: {error}"))?;
    let (tx, rx) = mpsc::channel::<Event>();

    let signal_tx = tx.clone();
    let _signal_thread = thread::spawn(move || {
        for raw_signal in signals.forever() {
            if signal_tx.send(Event::Signal(raw_signal)).is_err() {
                return;
            }
        }
    });

    if watch_parent {
        debug("parent watchdog enabled");
        let parent_pid = nix::unistd::getppid().as_raw();
        let _parent_thread = thread::spawn(move || loop {
            thread::sleep(PARENT_CHECK_INTERVAL);
            let current_parent = nix::unistd::getppid().as_raw();
            if current_parent != parent_pid || current_parent == 1 {
                if tx.send(Event::ParentGone).is_err() {
                    return;
                }
                return;
            }
        });
    }

    Ok(rx)
}

fn reap_child(child: &mut Child) {
    let start = Instant::now();
    while start.elapsed() < REAP_TIMEOUT {
        match child.try_wait() {
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Ok(Some(_)) | Err(_) => return,
        }
    }
}

fn debug(message: &str) {
    if env::var_os("OCLEAN_DEBUG").is_some() {
        eprintln!("oclean: {message}");
    }
}

fn exit_code_from_status(status: ExitStatus) -> ExitCode {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        if let Some(signal_number) = status.signal() {
            return exit_code_from_signal(signal_number);
        }
    }

    let code = status
        .code()
        .unwrap_or(1)
        .clamp(i32::from(u8::MIN), i32::from(u8::MAX));
    let code_u8 = u8::try_from(code).map_or(1, |value| value);
    ExitCode::from(code_u8)
}

fn exit_code_from_signal(signal_number: i32) -> ExitCode {
    let code = (128 + signal_number).clamp(i32::from(u8::MIN), i32::from(u8::MAX));
    let code_u8 = u8::try_from(code).map_or(1, |value| value);
    ExitCode::from(code_u8)
}

use std::collections::HashSet;
use std::thread;
use std::time::Duration;

use nix::sys::signal::{self, Signal};
use nix::unistd::{getpgid, getpid, Pid};

use crate::process_tree::{discover_descendants, pgid_exists, pid_exists, process_groups};

const POLL_INTERVAL: Duration = Duration::from_millis(150);
const TERM_GRACE: Duration = Duration::from_millis(1_200);
const SWEEP_PASSES: usize = 3;

pub struct Tracker {
    root_pid: i32,
    wrapper_pgid: i32,
    known_pids: HashSet<i32>,
    known_pgids: HashSet<i32>,
}

impl Tracker {
    pub fn new(root_pid: i32) -> Self {
        let wrapper_pgid = getpgid(Some(getpid())).map_or(-1, Pid::as_raw);

        Self {
            root_pid,
            wrapper_pgid,
            known_pids: HashSet::new(),
            known_pgids: HashSet::new(),
        }
    }

    pub fn observe(&mut self) {
        let descendants = discover_descendants(self.root_pid);
        self.known_pids.extend(descendants.iter().copied());

        for group in process_groups(&descendants) {
            if group <= 1 || group == self.wrapper_pgid {
                continue;
            }
            self.known_pgids.insert(group);
        }
    }

    fn live_pids(&self) -> HashSet<i32> {
        self.known_pids
            .iter()
            .copied()
            .filter(|pid| pid_exists(*pid))
            .collect()
    }

    fn live_pgids(&self) -> HashSet<i32> {
        self.known_pgids
            .iter()
            .copied()
            .filter(|group| pgid_exists(*group))
            .collect()
    }
}

pub fn post_exit_cleanup(tracker: &mut Tracker) {
    cleanup_tree_with_signal(tracker, Signal::SIGTERM, false);
    thread::sleep(POLL_INTERVAL);
    cleanup_tree_with_signal(tracker, Signal::SIGKILL, false);
}

pub fn cleanup_tree(tracker: &mut Tracker) {
    cleanup_tree_with_signal(tracker, Signal::SIGTERM, true);
    thread::sleep(TERM_GRACE);
    cleanup_tree_with_signal(tracker, Signal::SIGKILL, true);
}

fn cleanup_tree_with_signal(tracker: &mut Tracker, first_signal: Signal, kill_groups: bool) {
    for pass in 0..SWEEP_PASSES {
        tracker.observe();
        let pids = tracker.live_pids();
        let groups = tracker.live_pgids();
        if pids.is_empty() && groups.is_empty() {
            return;
        }

        let signal_kind = if pass == 0 {
            first_signal
        } else {
            Signal::SIGKILL
        };
        terminate_targets(&pids, &groups, signal_kind, kill_groups);
        thread::sleep(POLL_INTERVAL);
    }
}

fn terminate_targets(
    pids: &HashSet<i32>,
    groups: &HashSet<i32>,
    signal_kind: Signal,
    kill_groups: bool,
) {
    if kill_groups {
        for pgid in groups {
            if *pgid <= 1 {
                continue;
            }
            let _ = signal::kill(Pid::from_raw(-*pgid), signal_kind);
        }
    }

    for pid in pids {
        if *pid <= 1 {
            continue;
        }
        let _ = signal::kill(Pid::from_raw(*pid), signal_kind);
    }
}

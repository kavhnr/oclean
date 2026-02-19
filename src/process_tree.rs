use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Command;

use nix::errno::Errno;
use nix::sys::signal;
use nix::unistd::{getpgid, Pid};

pub fn process_groups(pids: &HashSet<i32>) -> HashSet<i32> {
    let mut groups = HashSet::new();
    for pid in pids {
        let nix_pid = Pid::from_raw(*pid);
        if let Ok(group) = getpgid(Some(nix_pid)) {
            groups.insert(group.as_raw());
        }
    }
    groups
}

pub fn pid_exists(pid: i32) -> bool {
    if pid <= 1 {
        return false;
    }

    !matches!(signal::kill(Pid::from_raw(pid), None), Err(Errno::ESRCH))
}

pub fn pgid_exists(pgid: i32) -> bool {
    if pgid <= 1 {
        return false;
    }

    !matches!(signal::kill(Pid::from_raw(-pgid), None), Err(Errno::ESRCH))
}

pub fn discover_descendants(root_pid: i32) -> HashSet<i32> {
    if root_pid <= 1 {
        return HashSet::new();
    }

    let Ok(parent_by_pid) = process_table() else {
        let mut single = HashSet::new();
        single.insert(root_pid);
        return single;
    };

    let mut children_by_parent: HashMap<i32, Vec<i32>> = HashMap::new();
    for (pid, parent) in parent_by_pid {
        children_by_parent.entry(parent).or_default().push(pid);
    }

    let mut descendants = HashSet::new();
    let mut queue = VecDeque::new();
    descendants.insert(root_pid);
    queue.push_back(root_pid);

    while let Some(parent) = queue.pop_front() {
        if let Some(children) = children_by_parent.get(&parent) {
            for child in children {
                if descendants.insert(*child) {
                    queue.push_back(*child);
                }
            }
        }
    }

    descendants
}

fn process_table() -> Result<HashMap<i32, i32>, String> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid="])
        .output()
        .map_err(|error| format!("failed to execute ps: {error}"))?;

    if !output.status.success() {
        return Err("ps command failed".to_owned());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut table = HashMap::new();

    for line in text.lines() {
        let mut columns = line.split_whitespace();
        let Some(pid_field) = columns.next() else {
            continue;
        };
        let Some(parent_field) = columns.next() else {
            continue;
        };

        let Ok(pid) = pid_field.parse::<i32>() else {
            continue;
        };
        let Ok(parent) = parent_field.parse::<i32>() else {
            continue;
        };

        table.insert(pid, parent);
    }

    Ok(table)
}

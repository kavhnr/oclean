use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

pub fn resolve_opencode_executable() -> Result<PathBuf, String> {
    if let Some(override_path) = env::var_os("OCLEAN_OPENCODE") {
        let resolved = PathBuf::from(override_path);
        if resolved.is_file() {
            return Ok(resolved);
        }
        return Err("OCLEAN_OPENCODE does not point to a file".to_owned());
    }

    let current_exe = env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    let current_canonical = current_exe
        .canonicalize()
        .unwrap_or_else(|_| current_exe.clone());
    let path_value = env::var_os("PATH").ok_or_else(|| "PATH is not set".to_owned())?;

    for dir in env::split_paths(&path_value) {
        let candidate = dir.join("opencode");
        if !candidate.is_file() {
            continue;
        }

        let canonical_candidate = candidate
            .canonicalize()
            .unwrap_or_else(|_| candidate.clone());
        if canonical_candidate == current_canonical {
            continue;
        }

        return Ok(candidate);
    }

    Err(
        "could not find real opencode binary in PATH (possible recursion); set OCLEAN_OPENCODE"
            .to_owned(),
    )
}

pub fn is_passthrough_mode(args: &[OsString]) -> bool {
    args.iter().any(|arg| {
        let value = arg.as_os_str();
        value == OsStr::new("--version")
            || value == OsStr::new("-v")
            || value == OsStr::new("--help")
            || value == OsStr::new("-h")
    })
}

#[cfg(test)]
mod tests {
    use super::is_passthrough_mode;
    use std::ffi::OsString;

    #[test]
    fn passthrough_detects_version_and_help_flags() {
        assert!(is_passthrough_mode(&[OsString::from("--version")]));
        assert!(is_passthrough_mode(&[OsString::from("-v")]));
        assert!(is_passthrough_mode(&[OsString::from("--help")]));
        assert!(is_passthrough_mode(&[OsString::from("-h")]));
    }

    #[test]
    fn passthrough_ignores_regular_commands() {
        assert!(!is_passthrough_mode(&[]));
        assert!(!is_passthrough_mode(&[OsString::from("run")]));
        assert!(!is_passthrough_mode(&[
            OsString::from("--model"),
            OsString::from("foo/bar")
        ]));
    }
}

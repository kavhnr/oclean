# oclean

`oclean` is a process-cleanup wrapper for `opencode` sessions.

It runs `opencode` as usual, tracks child processes created during that session, and cleans up session-owned leftovers when the session exits or is interrupted.

## Why this exists

`opencode` is moving quickly and improving constantly. `oclean` is a small companion utility for people who want stricter per-session process cleanup today.

Repository: https://github.com/kavhnr/oclean

## Install

### Cargo

```bash
cargo install oclean
```

### Homebrew

`oclean` is distributed via a custom tap.

```bash
brew tap kavhnr/oclean
brew install oclean
```

### GitHub Releases

Download a release artifact for your platform from the repo Releases page and place `oclean` on your `PATH`.

## Usage

Run `oclean` where you would normally run `opencode`:

```bash
oclean
```

Pass flags/args through as usual:

```bash
oclean --model provider/model
```

### Optional shell alias

```bash
alias opencode='oclean'
```

## Behavior and environment variables

- `OCLEAN_OPENCODE`: absolute path override for the real `opencode` binary.
- `OCLEAN_WATCH_PARENT=1`: enable parent-watchdog cleanup mode.
- `OCLEAN_DEBUG=1`: print lifecycle/debug events to stderr.

`oclean` includes recursion guards and will fail fast if it detects wrapper recursion.

## Publishing

### Crates.io

1. Update `Cargo.toml` metadata and version.
2. Run checks:

```bash
cargo clippy --all-targets --all-features
cargo test
```

3. Publish:

```bash
cargo publish
```

### GitHub Releases

Tag a version (`vX.Y.Z`) and push. The release workflow builds artifacts and uploads checksums.

### Homebrew tap

1. Create/update formula in tap repo (`homebrew-oclean`).
2. Point `url` to GitHub release tarball and set `sha256`.
3. Commit formula update and push.

## License

MIT

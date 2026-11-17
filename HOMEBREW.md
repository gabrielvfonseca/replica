# Homebrew

Replica is available through Homebrew via a tap of this repository.

## Install

```sh
brew tap gabrielvfonseca/replica https://github.com/gabrielvfonseca/replica
brew trust gabrielvfonseca/replica
brew install gabrielvfonseca/replica/replica
```

`brew trust` is required on Homebrew 7+ for third-party taps.

Or, if you prefer a local formula (no tap needed):

```sh
brew install ./Formula/replica.rb
```

## Build from source (HEAD)

```sh
brew install --HEAD gabrielvfonseca/replica/replica
# or
brew install --HEAD ./Formula/replica.rb
```

## Auto-update on new releases

The formula reads version and checksums from GitHub releases. When a new
release is published, update the formula by copying `version`, `url`, and
`sha256` from `bin/sha256sums.txt` (produced by `scripts/release.sh X.Y.Z`).

## Background GC service

```sh
brew services start replica
```

Runs `replica gc` every 10 minutes. Uses `REPLICA_ROOT=$HOME/replica` (launchd
doesn't read your shell profile). If your workspaces live elsewhere, run
`replica gc` from cron or your own launchd with the correct `REPLICA_ROOT`.

## Other ways to install

```sh
# Installer script: downloads the binary for your architecture and checks it against sha256sums.txt
curl -fsSL https://raw.githubusercontent.com/gabrielvfonseca/replica/main/scripts/get-replica.sh | sh

# From source (needs Rust: https://rustup.rs)
git clone https://github.com/gabrielvfonseca/replica.git && cd replica
make release && PREFIX=$HOME/.local make install
```

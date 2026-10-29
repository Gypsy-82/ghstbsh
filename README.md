# ghstbsh: a ghost shell

**Status:** v0.1.0 · alpha · unaudited · bash is the fully supported path (other
shells get a weaker, env-only fallback; see the threat model).

Post-compromise defense for your shell history. One job, done completely:
**your command history is never written to a file. Not on disk, not in
RAM-backed storage like tmpfs. Once your session ends there is nothing left to
recover.** Every history file is sent to `/dev/null`. An attacker who lands on
the box later, or a stolen/imaged disk, finds no history file to read, because
none was ever created.

In-session history still works (up-arrow): those commands live in the running
shell's memory for convenience and are discarded the instant it exits. That
in-memory list is readable by `root` (or the shell's parent process) *while the
session is live* (see the [threat model](#threat-model-what-it-covers-and-what-it-deliberately-doesnt)).
The guarantee is "nothing persists," not "invisible while running."

## Requirements

**To build:**
- A recent stable Rust toolchain (`cargo` / `rustc` 1.70+, edition 2021).
- **Zero external crate dependencies.** It compiles against the standard library
  alone, so there's nothing to fetch and no supply chain to vet.

**To run:**
- **Linux / Unix-like OS.** Uses Unix process APIs; it does not target Windows.
  Developed and tested on Linux.
- **`bash` on `PATH`.** The zero-file, clobber-proof launch execs `bash` and uses
  process substitution. A non-bash login shell still works via a weaker
  env-only fallback (see the threat model), but bash is the supported path.
- **No runtime dependencies, no daemon, no root.** Run it as yourself, or
  `sudo ghstbsh` for a root ghost shell. It is never installed setuid.

### Don't have Rust? (needed only to build from source)

The standard installer is `rustup` from the Rust project:

```bash
# official one-liner (installs cargo + rustc into ~/.cargo, no root)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"     # or just open a new terminal
cargo --version               # verify
```

Prefer not to pipe a script into a shell? Download it first and read it
(`curl -sSf https://sh.rustup.rs -o rustup.sh; less rustup.sh; sh rustup.sh`), or
install from your distro instead (often older, but fine here):

```bash
# Debian/Ubuntu            # Fedora                 # Arch
sudo apt install cargo     sudo dnf install cargo   sudo pacman -S rust
```

> **No Rust at all, just want the binary?** Skip this section entirely and use
> **Install, Option A** (prebuilt static binary, no toolchain required).

## Quick start

**Step 1: get `ghstbsh` onto your `PATH`. Pick ONE:**

```bash
# A) prebuilt binary (no Rust needed)
curl -LO https://github.com/Gypsy-82/ghstbsh/releases/latest/download/ghstbsh-x86_64-linux
curl -LO https://github.com/Gypsy-82/ghstbsh/releases/latest/download/ghstbsh-x86_64-linux.sha256
sha256sum -c ghstbsh-x86_64-linux.sha256                   # must print: ghstbsh-x86_64-linux: OK
install -Dm755 ghstbsh-x86_64-linux ~/.local/bin/ghstbsh

# B) build from source (needs Rust, see Requirements)
cargo build --release
install -Dm755 target/release/ghstbsh ~/.local/bin/ghstbsh
```

> `~/.local/bin` must be on your `$PATH` for the bare `ghstbsh` command to work.
> Not on it? Install system-wide instead: `sudo install -Dm755 <binary> /usr/local/bin/ghstbsh`.

**Steps 2 to 4: inspect, install the toggle, use it:**

```bash
# 2. (optional) inspect the exact launch; runs nothing
ghstbsh --dry-run

# 3. install the `ghst` toggle into your shell (review first, then append)
ghstbsh --print-function                                 # read it
ghstbsh --print-function >> ~/.bashrc && source ~/.bashrc

# 4. use it
ghstbsh            # drop into a ghost shell: all history -> /dev/null
ghst on            # or: silence the shell you're ALREADY in
ghst off           # restore
ghst status        # ON (recording) / OFF (silent)
```

Inside a ghost shell, confirm it's live: `echo "$HISTFILE"` prints `/dev/null`,
up-arrow still recalls this session's commands, and nothing survives `exit`.

## What it is / isn't

- **Is:** total shell-history silence, on demand. Keeps secrets typed inline
  (`mysql -pXYZ`, `export AWS_SECRET=...`) and your lateral-movement map (which
  hosts you SSH to) out of every history file: shell, plus `less`, `mysql`,
  `psql`, `python`, `node`, `redis-cli`.
- **Isn't:** protection against an attacker *already live as root*, or against
  `auditd` / `sudo` logs / `utmp` / any server-side logging an admin enforces.
  On boxes you don't own, this is "don't leave my passwords in a flat file," not
  "be invisible." Suppressing history on systems you don't administer may
  violate their policy.

## Threat model: what it covers, and what it deliberately doesn't

ghstbsh makes one guarantee, and makes it completely:

> **Your shell/REPL history is never written to any file: not on disk, not in
> RAM-backed storage, so after your session there is nothing to recover.**

Verified: after a ghost session, no history lands in any dotfile, in `$HOME`,
in `/tmp` `/var/tmp` `/dev/shm` `/run/user`, and once the shell exits its memory
is reclaimed. That's the whole promise, and it's airtight.

It is **not** an invisibility cloak, and does not claim to be. Honestly outside
its scope:

- **A live session's memory.** While a ghost shell is running, the commands you
  typed are in that process's RAM. That is exactly what makes up-arrow work.
  **root**, or the shell's **parent process**, can read that memory. Nothing a
  history tool can prevent without disabling in-session history; if you want
  that, use `ghst on` (`set +o history`), which turns the list off entirely.
- **Swap / hibernation.** If the host uses *disk-backed* swap or hibernates, the
  kernel may page that in-memory history to persistent disk. RAM-backed swap
  (zram) doesn't spill. Mitigate with encrypted swap or disabled hibernation.
- **System-level logging.** `auditd`, `pam_tty_audit`, `sudo` (logged to
  `auth.log`), and preload shims like snoopy record commands independently of
  your shell. ghstbsh does **not** touch these, by design: a history tool that
  disabled system auditing would be anti-forensics tooling, not hygiene. Tuning
  sudoers or audit rules is an owner decision you make in system config,
  separately, and with eyes open. `auth.log` is also how *you'd* detect and
  reconstruct a real intrusion, so prefer a toggle over a permanent gag.
- **The terminal layer.** Terminal-emulator scrollback, `tmux`/`screen` capture,
  `script`, and SSH-client logging all live outside the shell and can retain
  what was on screen.
- **Secrets in arguments.** `mysql -pHUNTER2` is visible via `ps`/`/proc` to
  others *while it runs*, regardless of history. Use `HISTCONTROL=ignorespace`
  (leading space), password prompts, and secret managers.

## Why `/dev/null` everywhere (and no RAM/tmpfs mode)

An earlier design kept history in a RAM tmpfs file for cross-session
convenience. That meant a history file *existed* during your session, readable
by you or root while logged in. `/dev/null` means **there is no file to read, at
any moment, ever.** For "nothing saved anywhere," that's strictly stronger, so
it's the only mode.

`shred` is likewise absent on purpose: it can't guarantee block overwrite on
SSD/CoW/overlay filesystems anyway, and "never written" beats "written then
scrubbed."

## Install

### Option A: prebuilt binary (no Rust needed)

Grab the static binary from the [Releases page](https://github.com/Gypsy-82/ghstbsh/releases),
check it, and put it on your `PATH`:

```bash
curl -LO https://github.com/Gypsy-82/ghstbsh/releases/latest/download/ghstbsh-x86_64-linux
curl -LO https://github.com/Gypsy-82/ghstbsh/releases/latest/download/ghstbsh-x86_64-linux.sha256
sha256sum -c ghstbsh-x86_64-linux.sha256         # must print: ghstbsh-x86_64-linux: OK
install -Dm755 ghstbsh-x86_64-linux ~/.local/bin/ghstbsh
```

One self-contained file, with no toolchain and no shared-library requirements,
runs on any x86-64 Linux. Always verify the checksum before trusting a security
tool you downloaded. *(No release published yet? Use Option B.)*

### Option B: build from source

```bash
cargo build --release                                               # needs Rust (see Requirements)
install -Dm755 target/release/ghstbsh ~/.local/bin/ghstbsh          # your user only (no sudo)
sudo install -Dm755 target/release/ghstbsh /usr/local/bin/ghstbsh   # OR system-wide (incl. root)
```

Install location sets only *where the binary lives*, never what it can do. Run
it as yourself for a ghost shell as you, or `sudo ghstbsh` for one as root. Same
tool, history to `/dev/null` either way. It is never setuid; elevate with sudo
when you need to.

## Use

```
ghstbsh                 # drop into a ghost shell (all history -> /dev/null)
ghstbsh --dry-run       # show exactly what would run; launch nothing
ghstbsh --print-function >> ~/.bashrc && source ~/.bashrc
```

### The `ghst` toggle (for the shell you're ALREADY in)

A compiled binary is a *child process*, so it cannot gag the shell that launched
it (a child can't modify its parent's shell state). So the in-place toggle is a
shell **function**, installed via `--print-function`:

```
ghst on        # silence THIS shell now (history off, up-arrow off)
ghst off       # restore
ghst status    # am I recording?
ghst           # hand off to a fresh ghost shell
```

It also sets `HISTCONTROL=ignorespace:ignoredups`, so any command you start with
a **leading space** is never recorded, the only reliable per-command silence.

## How it beats the dotfile-clobber gotcha, with zero files

The ghost settings ride in on a **process-substitution rcfile that lives in a
kernel pipe, never a filesystem**:

```
bash --rcfile <(cat ~/.bashrc ; <our /dev/null exports>) -i
```

The `<()` hands bash a `/dev/fd/NN` path backed by an in-kernel pipe: not a
temp file, not tmpfs, nothing that touches storage. Bash sources your
`~/.bashrc` first (you keep your aliases and prompt), then our exports fire
**last**, so no dotfile can quietly re-point `HISTFILE` back to a real file, and
we never wrote a byte to disk to make that happen. That's what makes the
"nothing saved anywhere" claim airtight: even the *config* that enforces the
silence is never a file.

**Footnote: this zero-file guarantee is the bash path specifically.** A
non-bash shell falls back to env-only: the exports are set in the environment
and the shell is launched with `-i`, but a `~/.zshrc` that re-points `HISTFILE`
could still override them. bash gets the ironclad treatment; other shells get
best-effort.

## One-time cleanup of PRE-EXISTING history

`ghstbsh` governs everything from now on. It does **not** remove the
`~/.bash_history` already on your disk. Clean that once, by hand:

```
cat ~/.bash_history          # review it first if you like
rm -f ~/.bash_history ~/.zsh_history ~/.lesshst ~/.mysql_history \
      ~/.psql_history ~/.python_history ~/.node_repl_history ~/.rediscli_history
```

Plain `rm` is correct: since `shred` can't guarantee overwrite on SSD/CoW
anyway, removing the file is what defeats a later `cat`.

## License

MIT. See [LICENSE](LICENSE).

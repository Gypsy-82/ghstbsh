//! ghstbsh — a ghost shell.
//!
//! Launches an interactive shell in which command history is sent to /dev/null.
//! Nothing is written to any file, in any location — not disk, not even a RAM
//! tmpfs. In-session history (up-arrow) still works, because that list lives in
//! the shell's memory and is discarded on exit.
//!
//! This binary is the *launcher*. It spawns a NEW, clean child shell — it does
//! NOT and CANNOT gag the shell that invoked it (a child process can't modify
//! its parent's shell state; that's what the `ghst` shell function is for).
//!
//! How it beats the ~/.bashrc clobber WITHOUT writing a file: for bash it execs
//!     bash --rcfile <(cat ~/.bashrc ; <our /dev/null exports>) -i
//! The <() is a kernel pipe (/dev/fd/NN), not a filesystem file. We source the
//! user's own rc first, then force the history settings LAST so nothing in their
//! dotfiles can re-point HISTFILE back to a real file.
//!
//! Modes:
//!   ghstbsh                 launch the ghost shell (history -> /dev/null)
//!   ghstbsh --dry-run       print exactly what WOULD run; launch nothing
//!   ghstbsh --print-function  emit the `ghst` shell function for your ~/.bashrc
//!   ghstbsh --help

use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;

const HELP: &str = "\
ghstbsh — launch a shell whose history is never saved anywhere

USAGE:
    ghstbsh [OPTION]

OPTIONS:
    (none)              launch the ghost shell: all history -> /dev/null
    --dry-run           show exactly what would run; launch nothing
    --print-function    print the `ghst` shell function to paste into ~/.bashrc
    -V, --version       print version and exit
    -h, --help          this text

Nothing is written to disk or to RAM. In-session up-arrow still works and is
discarded when the shell exits. To silence the shell you are CURRENTLY in (not a
fresh one), use the `ghst on` / `ghst off` function — see --print-function.
";

/// Every history-file variable we null out, plus the in-memory knobs that keep
/// up-arrow usable while persisting nothing. Order matters: these are emitted
/// AFTER the user's rc, so they win.
const EXPORTS: &[&str] = &[
    "export HISTFILE=/dev/null",          // bash / zsh / sh
    "export HISTFILESIZE=0",              // never write the bash history file
    "export HISTSIZE=5000",               // in-memory list -> up-arrow works
    "export HISTCONTROL=ignorespace:ignoredups",
    "export LESSHISTFILE=/dev/null",      // less
    "export MYSQL_HISTFILE=/dev/null",    // mysql client
    "export PSQL_HISTORY=/dev/null",      // psql
    "export PYTHONHISTORY=/dev/null",     // python 3.13+
    "export NODE_REPL_HISTORY=",          // node: empty = disabled
    "export REDISCLI_HISTFILE=/dev/null", // redis-cli
];

/// Visual cue so you always know you're in a ghost shell. Prepends a bold
/// neon-pink (256-color 198) `(ghst)` tag to the existing prompt. The `\[ \]`
/// wrap the escapes as zero-width so the prompt never miscounts its length and
/// corrupts on long lines / history scroll. Cosmetic only — does not touch the
/// history mechanism. Runs AFTER ~/.bashrc, so it wins; ${PS1} keeps your prompt.
const PROMPT: &str = r#"PS1="\[\e[1;38;5;198m\](ghst)\[\e[0m\] ${PS1}""#;

/// All lines we append to the rcfile, after the user's ~/.bashrc: history
/// nulling first, then the prompt marker.
fn rcfile_lines() -> Vec<&'static str> {
    EXPORTS.iter().copied().chain(std::iter::once(PROMPT)).collect()
}

/// Build the single-quoted argument list for `printf '%s\n' …` so each export
/// lands on its own line inside the process-substitution rcfile. Our export
/// strings contain no single quotes, so plain single-quoting is safe.
fn exports_as_printf_args() -> String {
    rcfile_lines()
        .iter()
        .map(|e| format!("'{e}'"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The command handed to `bash -c`: re-exec an interactive bash whose rcfile is
/// a kernel pipe sourcing ~/.bashrc then our exports. Zero files touched.
fn bash_launch_cmd() -> String {
    format!(
        "exec bash --rcfile <(cat \"$HOME/.bashrc\" 2>/dev/null; printf '%s\\n' {}) -i",
        exports_as_printf_args()
    )
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut dry = false;

    for a in &args {
        match a.as_str() {
            "--dry-run" => dry = true,
            "--print-function" => {
                print!("{}", ghst_function());
                return;
            }
            "-V" | "--version" => {
                println!("ghstbsh {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-h" | "--help" => {
                print!("{HELP}");
                return;
            }
            other => {
                eprintln!("ghstbsh: unknown argument: {other}\n");
                print!("{HELP}");
                std::process::exit(2);
            }
        }
    }

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let is_bash = shell.rsplit('/').next() == Some("bash");

    if dry {
        println!("ghstbsh --dry-run  (nothing launched, nothing written)\n");
        println!("  behavior   : all command history -> /dev/null (saved nowhere)");
        println!("  your shell : {shell}");
        println!("  exports    :");
        for e in EXPORTS {
            println!("      {e}");
        }
        println!("  prompt cue :");
        println!("      {PROMPT}");
        if is_bash {
            println!("\n  launches   : bash -c '<zero-disk rcfile launch>'");
            println!("  exact cmd  : {}", bash_launch_cmd());
            println!("\n  the rcfile is a kernel pipe <() — NOT a file on any filesystem.");
            println!("  it sources ~/.bashrc first, then forces the exports (beats clobber).");
        } else {
            println!("\n  launches   : {shell} -i  with the exports set in the environment");
            println!("  note       : non-bash shell — env-only; a ~/.{{sh,zsh}}rc that re-points");
            println!("               HISTFILE could override. bash gets the clobber-proof path.");
        }
        println!("\n  files written: NONE");
        return;
    }

    // Real launch. exec() replaces this process with the shell: clean tty, no
    // lingering parent.
    let err = if is_bash {
        Command::new("bash").arg("-c").arg(bash_launch_cmd()).exec()
    } else {
        let mut cmd = Command::new(&shell);
        for e in EXPORTS {
            if let Some((k, v)) = e.trim_start_matches("export ").split_once('=') {
                cmd.env(k, v);
            }
        }
        cmd.arg("-i").exec()
    };

    eprintln!("ghstbsh: failed to launch {shell}: {err}");
    std::process::exit(1);
}

/// The `ghst` shell function — the IN-PLACE toggle for the current shell.
/// Must be sourced into the shell (a binary can't modify its parent), so we
/// emit it as text to paste or append into ~/.bashrc.
fn ghst_function() -> String {
    "\
# >>> ghst >>>  (ghost shell helper — paste into ~/.bashrc, then: source ~/.bashrc)
export HISTCONTROL=ignorespace:ignoredups   # leading-space commands never recorded
ghst() {
  case \"$1\" in
    on)       set +o history; unset HISTFILE
              echo 'ghst: history OFF for this shell (up-arrow also off)';;
    off)      export HISTFILE=\"$HOME/.bash_history\"; set -o history
              echo 'ghst: history ON';;
    status)   if set -o | grep -q 'history[[:space:]]*on'; then echo 'ghst: ON (recording)';
              else echo 'ghst: OFF (silent)'; fi;;
    ''|shell) exec ghstbsh;;             # drop into a ghost shell (history -> /dev/null)
    *)        echo 'usage: ghst [on|off|status|shell]';;
  esac
}
# <<< ghst <<<
"
    .to_string()
}

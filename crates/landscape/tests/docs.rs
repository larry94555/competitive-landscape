//! The README is executable.
//!
//! Two bugs reached a reader that no unit test could have caught, because both were in
//! prose rather than code:
//!
//! 1. The README said port 8080. The binary had moved to 8787. `curl` reached a different
//!    program entirely and returned its 404.
//! 2. A documented `curl` omitted `-H 'content-type: application/json'`, so following the
//!    instructions produced a rejection. Every API test passed, because every API test used
//!    a helper that always set the header — the helper hid the failure mode a reader hits.
//!
//! In both cases the code was correct and the instructions were wrong. The only thing that
//! catches that is running the instructions.
//!
//! So: this test parses `README.md`, boots the real binary, and runs every documented
//! command against it. A command that no longer works fails the build.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

const README: &str = include_str!("../../../README.md");

/// Extract shell commands from fenced ```bash blocks, joining `\` continuations.
fn documented_commands() -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    let mut pending = String::new();

    for line in README.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            inside = trimmed.starts_with("```bash");
            continue;
        }
        if !inside || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(head) = trimmed.strip_suffix('\\') {
            pending.push_str(head.trim_end());
            pending.push(' ');
            continue;
        }
        pending.push_str(trimmed);
        out.push(std::mem::take(&mut pending));
    }
    out
}

// --------------------------------------------------------------------- static checks

#[test]
fn the_readme_port_matches_the_binary_default() {
    // This is the whole of bug 1, as a test that needs nothing running. The number in the
    // prose and the number in the code are two copies of one fact, so something has to
    // hold them together.
    let main_rs = include_str!("../src/main.rs");
    let default = main_rs
        .lines()
        .find(|l| l.contains("const DEFAULT_ADDR"))
        .and_then(|l| l.split('"').nth(1))
        .expect("DEFAULT_ADDR is declared as a string literal");

    let port = default.rsplit(':').next().expect("DEFAULT_ADDR has a port");

    let mut checked = 0;
    for command in documented_commands() {
        // Only commands addressing *our* HTTP surface. The README also names Postgres on
        // 5432 and Vite on 5173, which are correctly not our port.
        if !command.contains("/api/") {
            continue;
        }
        checked += 1;
        assert!(
            command.contains(port),
            "README command talks to /api/ on a port the binary does not listen on \
             (expected {port}): {command}"
        );
    }
    assert!(
        checked > 0,
        "no README command addresses /api/, so this test is checking nothing"
    );
}

#[test]
fn every_documented_cargo_command_is_a_real_role() {
    // Catches a README that still tells you to run a subcommand that was renamed.
    let main_rs = include_str!("../src/main.rs");
    for command in documented_commands() {
        let Some(rest) = command.strip_prefix("cargo run -- ") else {
            continue;
        };
        let role = rest.split_whitespace().next().unwrap_or_default();
        assert!(
            main_rs.contains(&format!("Some(\"{role}\")")),
            "README documents `cargo run -- {role}`, which the binary does not accept"
        );
    }
}

#[test]
fn every_documented_post_sets_a_content_type() {
    // This is bug 2, caught without a server. A POST with a body and no content-type is
    // rejected, so documenting one is documenting a failure.
    for command in documented_commands() {
        if !command.contains("curl") || !command.contains(" -d ") {
            continue;
        }
        assert!(
            command.to_lowercase().contains("content-type"),
            "a documented POST sends a body without a content-type header: {command}"
        );
    }
}

// --------------------------------------------------------------- the live check

/// A running server, stopped when it goes out of scope.
///
/// The guard owns the child from the moment it is spawned, so an assertion that panics
/// mid-test still leaves nothing running — and neither does a failure to start.
struct Server {
    child: Child,
    port: u16,
}

impl Server {
    /// Start the binary on an OS-assigned port and wait until it reports which one.
    ///
    /// An OS-assigned port rather than a fixed one because these tests must not care what
    /// else the machine is doing — a fixed port is how the whole llama-server collision
    /// started.
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_landscape"))
            .args(["dev", "--store", "memory"])
            .env("BIND_ADDR", "127.0.0.1:0")
            .env("RUST_LOG", "landscape=info")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary should start");

        // Logs go to stderr. Reading the bound port back from them is why serve() reports
        // local_addr rather than the address it was asked for.
        let stderr = child.stderr.take().expect("stderr is piped");
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        let mut seen = String::new();

        for _ in 0..40 {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            seen.push_str(&line);
            let Some(idx) = line.find("listening on http://") else {
                continue;
            };
            let addr = line[idx + "listening on http://".len()..].trim();
            let port = addr
                .rsplit(':')
                .next()
                .and_then(|p| p.trim().parse::<u16>().ok());
            match port {
                Some(port) => {
                    // Keep draining. Dropping the reader closes the read end of the pipe,
                    // and the server dies on its next log write — which looked exactly
                    // like "curl cannot connect" and took a while to attribute.
                    std::thread::spawn(move || {
                        let mut sink = String::new();
                        while reader.read_line(&mut sink).unwrap_or(0) > 0 {
                            sink.clear();
                        }
                    });
                    return Self { child, port };
                }
                None => {
                    let mut failed = Self { child, port: 0 };
                    failed.stop();
                    panic!("could not read a port from {addr:?}");
                }
            }
        }

        let mut failed = Self { child, port: 0 };
        failed.stop();
        panic!("the server never reported a listening address. It said:\n{seen}");
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

#[test]
#[ignore = "spawns the binary and shells out to curl; run with --ignored or in CI"]
fn every_documented_curl_actually_works() {
    let server = Server::start();

    let commands: Vec<String> = documented_commands()
        .into_iter()
        .filter(|c| c.starts_with("curl"))
        .collect();

    assert!(
        commands.len() >= 2,
        "expected the README to document at least the health check and a POST, found {}",
        commands.len()
    );

    for command in commands {
        // Point the documented command at this run's port, changing nothing else.
        let runnable = command
            .replace("127.0.0.1:8787", &format!("127.0.0.1:{}", server.port))
            .replace("localhost:8787", &format!("127.0.0.1:{}", server.port));

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("{runnable} -w '\\n%{{http_code}}'"))
            .output()
            .expect("curl should run - is it on PATH?");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let (body, status) = stdout.rsplit_once('\n').unwrap_or((stdout.as_ref(), ""));

        assert!(
            output.status.success(),
            "documented command failed to execute:\n  {runnable}\n  {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // A documented command must not produce a client error. That is exactly what both
        // reported bugs looked like from the reader's side: a 404 from the wrong server,
        // and a 400 from a missing header.
        assert!(
            !status.starts_with('4') && !status.starts_with('5'),
            "a documented command returns HTTP {status}, so following the README fails:\
             \n  {runnable}\n  {body}"
        );

        assert!(
            serde_json::from_str::<serde_json::Value>(body.trim()).is_ok(),
            "a documented command returned something that is not JSON:\n  {runnable}\n  {body}"
        );
    }
}

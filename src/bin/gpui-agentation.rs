#[path = "../bridge/mod.rs"]
mod bridge;

use std::{io, process::ExitCode, time::Duration};

use bridge::{Bridge, Config};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut url =
        std::env::var("GPUI_AGENTATION_URL").unwrap_or_else(|_| "http://127.0.0.1:4748/mcp".into());
    let mut timeout = Duration::from_secs(10);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--url" => url = args.next().ok_or("--url requires a value")?,
            "--timeout-ms" => {
                let milliseconds: u64 = args
                    .next()
                    .ok_or("--timeout-ms requires a value")?
                    .parse()
                    .map_err(|_| "timeout must be an integer in 1..=60000")?;
                if !(1..=60_000).contains(&milliseconds) {
                    return Err("timeout must be an integer in 1..=60000".into());
                }
                timeout = Duration::from_millis(milliseconds);
            }
            "--help" | "-h" => {
                println!(
                    "gpui-agentation [--url http://127.0.0.1:PORT/mcp] [--timeout-ms 10000]\n\nRelays newline-delimited stdio MCP to a running local Agentation app.\nDefault URL: http://127.0.0.1:4748/mcp (or GPUI_AGENTATION_URL).\nOnly literal loopback HTTP endpoints are accepted. No API key is needed."
                );
                return Ok(());
            }
            "--version" | "-V" => {
                println!("gpui-agentation {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => return Err("unknown option; use --help".into()),
        }
    }
    let config = Config::new(&url, timeout)?;
    Bridge::new(config)?.serve(io::stdin().lock(), io::stdout().lock())?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gpui-agentation: {error}");
            ExitCode::FAILURE
        }
    }
}

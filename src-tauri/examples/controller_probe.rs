use std::{
    error::Error,
    thread,
    time::{Duration, Instant},
};

use dual_deck_lib::controller::{ControllerConfig, ControllerRuntime};

const DEFAULT_DURATION_MS: u64 = 5_000;

fn main() -> Result<(), Box<dyn Error>> {
    let duration = probe_duration();
    let mut runtime = ControllerRuntime::start(ControllerConfig::default())?;
    let deadline = Instant::now() + duration;

    while Instant::now() < deadline {
        match runtime.events().try_recv() {
            Ok(event) => println!("{}", serde_json::to_string(&event)?),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&runtime.handle().snapshot())?
    );
    Ok(())
}

fn probe_duration() -> Duration {
    let milliseconds = std::env::args()
        .skip_while(|argument| argument != "--duration-ms")
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| {
            std::env::var("DUALDECK_PROBE_DURATION_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
        })
        .unwrap_or(DEFAULT_DURATION_MS);
    Duration::from_millis(milliseconds)
}

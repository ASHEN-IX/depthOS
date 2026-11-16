use depthos_observer::{ObservationHealth, ObservedStateSource, Receiver};
#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!(
            "Observer receiver failed; check the session bus and whether another receiver owns its name."
        );
        std::process::exit(1);
    }
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("inspect");
    if !matches!(command, "inspect" | "health" | "snapshot")
        || args.iter().skip(1).any(|a| a != "--redact-titles")
    {
        eprintln!("Usage: depthos-observer [inspect|health|snapshot] [--redact-titles]");
        return Ok(());
    }
    let receiver = Receiver::session().await?;
    let observer = receiver.observer();
    let mut updates = observer.subscribe();
    eprintln!("Waiting for an explicitly launched KWin observer; Ctrl+C stops this receiver.");
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = sigterm.recv() => break,
            result = updates.changed() => {
                if result.is_err() { break; }
                if command == "snapshot" && observer.health() == ObservationHealth::Synchronized {
                    let mut snapshot = observer.snapshot()?; snapshot.redact_titles();
                    println!("{}", serde_json::to_string_pretty(&snapshot)?); break;
                } else if command != "snapshot" { println!("{:?}", observer.health()); }
            }
        }
    }
    receiver.shutdown().await?;
    Ok(())
}

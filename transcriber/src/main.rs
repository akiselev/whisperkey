fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Transcriber stub started");
    // For now, just sleep so the process stays alive
    std::thread::sleep(std::time::Duration::from_secs(60));
}

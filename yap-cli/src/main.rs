mod app;
mod cli;
mod commands;
mod input;
mod output;

#[tokio::main]
async fn main() {
    if let Err(error) = app::run().await {
        eprintln!("Fatal error: {error}");
    }
}
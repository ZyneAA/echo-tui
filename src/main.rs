mod app;
mod awdio;
mod config;
mod result;
mod ignite;
mod ui;

#[tokio::main]
async fn main() -> result::EchoResult<()> {
    match ignite::engine() {
        Ok(val) => {
            if let Err(e) = app::start(val).await {
                eprintln!("{}", e);
            }
        }
        Err(e) => eprintln!("{}", e),
    }

    Ok(())
}

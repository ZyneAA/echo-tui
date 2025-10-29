use std::io;

mod app;
mod config;
mod ignite;
mod ui;

#[tokio::main]
async fn main() -> io::Result<()> {
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

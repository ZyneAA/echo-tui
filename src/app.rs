use std::{io, path::PathBuf};
use std::sync;

use crate::config::Config;

use super::ui;

#[derive(Debug, Default)]
pub struct State {
    exit: bool,
}

impl State {
    pub async fn init(&mut self, data: (Config, PathBuf)) -> io::Result<()> {
        let mut canvas = ui::Canvas::init(data.0);

        let exit_flag = sync::Arc::new(sync::atomic::AtomicBool::new(self.exit));
        let exit_clone = exit_flag.clone();

        tokio::spawn(async move {
            if let Err(e) = canvas.render(exit_clone).await {
                eprintln!("Canvas render error: {e}");
            }
        });

        ratatui::restore();
        Ok(())
    }
}

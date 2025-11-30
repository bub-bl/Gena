mod app;
mod editor_window;

use anyhow::Result;

use crate::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.init()?;

    Ok(())
}

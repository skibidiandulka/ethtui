use crate::app::App;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub async fn handle_key_events(key_event: KeyEvent, app: &mut App) -> Result<()> {
    match key_event.code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Esc => app.quit(),
        KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => app.quit(),

        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),

        KeyCode::Char('r') => {
            // Force refresh now.
            app.tick().await?;
        }

        KeyCode::Char('n') => {
            if let Err(e) = app.renew_dhcp().await {
                app.last_error = Some(e.to_string());
            } else {
                let _ = app.tick().await;
            }
        }

        _ => {}
    }

    Ok(())
}

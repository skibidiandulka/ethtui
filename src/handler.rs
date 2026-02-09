use crate::{
    app::{App, ToastKind},
    net::EthernetDevice,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn snap(d: &EthernetDevice) -> String {
    let carrier = d.carrier.map(|c| if c { "1" } else { "0" }).unwrap_or("?");
    let ip = d.ipv4.first().cloned().unwrap_or_else(|| "-".into());
    let gw = d.gateway_v4.clone().unwrap_or_else(|| "-".into());
    let dns = if d.dns.is_empty() {
        "-".to_string()
    } else {
        d.dns.join(", ")
    };
    format!(
        "state={}; carrier={}; ip={}; gw={}; dns={}",
        d.operstate, carrier, ip, gw, dns
    )
}

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
            app.clear_error();
            let (iface, before) = match app.selected_device() {
                Some(d) => (d.name.clone(), snap(d)),
                None => ("-".to_string(), "no interface selected".to_string()),
            };

            match app.renew_dhcp().await {
                Ok(out) => {
                    // Refresh state after the command returns so UI reflects any new lease/IP.
                    let _ = app.tick().await;
                    let after = app
                        .selected_device()
                        .map(snap)
                        .unwrap_or_else(|| "no interface selected".to_string());

                    let mut msg = format!("{iface}: DHCP renew requested");
                    if out.used_sudo {
                        msg.push_str(" (sudo)");
                    }
                    if !out.stdout.is_empty() || !out.stderr.is_empty() {
                        let mut extra = String::new();
                        if !out.stdout.is_empty() {
                            extra.push_str(&format!("stdout: {}", out.stdout));
                        }
                        if !out.stderr.is_empty() {
                            if !extra.is_empty() {
                                extra.push('\n');
                            }
                            extra.push_str(&format!("stderr: {}", out.stderr));
                        }
                        msg.push_str(&format!("\n{}", extra));
                    }

                    if before == after {
                        msg.push_str("\nNo change detected (lease may still have been renewed).");
                    }
                    msg.push_str(&format!("\nBefore: {before}\nAfter:  {after}"));

                    app.set_toast(ToastKind::Success, msg);
                    let body = if before == after {
                        "DHCP renew requested (no visible change)."
                    } else {
                        "DHCP renew requested."
                    };
                    app.notify("󰀂    Ethernet", &format!("{iface}: {body}"))
                        .await;
                }
                Err(e) => {
                    app.last_error = Some(e.to_string());
                    app.set_toast(ToastKind::Error, format!("{iface}: DHCP renew failed"));
                    app.notify("󰀂    Ethernet", &format!("{iface}: DHCP renew failed"))
                        .await;
                }
            }
        }

        _ => {}
    }

    Ok(())
}

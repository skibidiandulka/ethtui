use crate::net::{EthernetDevice, list_ethernet_devices};
use anyhow::Result;
use ratatui::widgets::TableState;
use tokio::process::Command;

pub struct App {
    pub running: bool,
    pub devices: Vec<EthernetDevice>,
    pub devices_state: TableState,
    pub last_error: Option<String>,
    pub last_action: Option<String>,
}

impl App {
    pub async fn new() -> Result<Self> {
        let devices = list_ethernet_devices()?;
        let mut devices_state = TableState::default();
        if devices.is_empty() {
            devices_state.select(None);
        } else {
            devices_state.select(Some(0));
        }

        Ok(Self {
            running: true,
            devices,
            devices_state,
            last_error: None,
            last_action: None,
        })
    }

    pub async fn tick(&mut self) -> Result<()> {
        // Refresh state periodically so link/IP changes show up without restarting the TUI.
        match list_ethernet_devices() {
            Ok(devices) => {
                let selected = self.devices_state.selected();
                self.devices = devices;
                if self.devices.is_empty() {
                    self.devices_state.select(None);
                } else if let Some(i) = selected {
                    self.devices_state.select(Some(i.min(self.devices.len() - 1)));
                } else {
                    self.devices_state.select(Some(0));
                }
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(e.to_string());
            }
        }

        Ok(())
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn select_next(&mut self) {
        if self.devices.is_empty() {
            self.devices_state.select(None);
            return;
        }

        let i = match self.devices_state.selected() {
            Some(i) => (i + 1).min(self.devices.len() - 1),
            None => 0,
        };
        self.devices_state.select(Some(i));
    }

    pub fn select_prev(&mut self) {
        if self.devices.is_empty() {
            self.devices_state.select(None);
            return;
        }

        let i = match self.devices_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.devices_state.select(Some(i));
    }

    pub fn selected_device(&self) -> Option<&EthernetDevice> {
        self.devices_state.selected().and_then(|i| self.devices.get(i))
    }

    fn selected_iface(&self) -> Result<String> {
        self.selected_device()
            .map(|d| d.name.clone())
            .ok_or_else(|| std::io::Error::other("no interface selected").into())
    }

    async fn run_privileged(&mut self, program: &str, args: &[&str]) -> Result<()> {
        // Try without sudo first (works if running as root or with capabilities).
        let output = Command::new(program).args(args).output().await;
        if let Ok(output) = output {
            if output.status.success() {
                return Ok(());
            }
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            // Permission errors are common. Retry with non-interactive sudo.
            if stderr.contains("Operation not permitted")
                || stderr.contains("Permission denied")
                || output.status.code() == Some(1)
            {
                let sudo_out = Command::new("sudo")
                    .arg("-n")
                    .arg(program)
                    .args(args)
                    .output()
                    .await?;
                if sudo_out.status.success() {
                    return Ok(());
                }
                let sudo_err = String::from_utf8_lossy(&sudo_out.stderr).trim().to_string();
                return Err(std::io::Error::other(if sudo_err.is_empty() {
                    format!("{} failed (sudo)", program)
                } else {
                    sudo_err
                })
                .into());
            }

            return Err(std::io::Error::other(if stderr.is_empty() {
                format!("{} failed", program)
            } else {
                stderr
            })
            .into());
        }

        // If spawning failed, bubble up.
        let output = output?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other("command failed").into())
        }
    }

    pub async fn link_up(&mut self) -> Result<()> {
        let iface = self.selected_iface()?;
        self.run_privileged("ip", &["link", "set", "dev", &iface, "up"])
            .await?;
        self.last_action = Some(format!("Brought {iface} up"));
        Ok(())
    }

    pub async fn link_down(&mut self) -> Result<()> {
        let iface = self.selected_iface()?;
        self.run_privileged("ip", &["link", "set", "dev", &iface, "down"])
            .await?;
        self.last_action = Some(format!("Brought {iface} down"));
        Ok(())
    }

    pub async fn renew_dhcp(&mut self) -> Result<()> {
        let iface = self.selected_iface()?;
        // systemd-networkd environments: try `networkctl renew`, else fall back to `reconfigure`.
        let out = Command::new("networkctl")
            .arg("renew")
            .arg(&iface)
            .output()
            .await;

        if let Ok(out) = out {
            if out.status.success() {
                self.last_action = Some(format!("Renewed DHCP on {iface}"));
                return Ok(());
            }
            let err = String::from_utf8_lossy(&out.stderr);
            if err.contains("Unknown") || err.contains("invalid") {
                self.run_privileged("networkctl", &["reconfigure", &iface])
                    .await?;
                self.last_action = Some(format!("Reconfigured {iface}"));
                return Ok(());
            }
        }

        self.run_privileged("networkctl", &["renew", &iface]).await?;
        self.last_action = Some(format!("Renewed DHCP on {iface}"));

        // Best-effort desktop notification (Omarchy uses mako).
        // If notify-send isn't available or there's no session bus, ignore.
        let _ = Command::new("notify-send")
            .arg("󰀂    Ethernet")
            .arg(format!("Renewed DHCP on {iface}"))
            .arg("-t")
            .arg("2000")
            .output()
            .await;
        Ok(())
    }
}

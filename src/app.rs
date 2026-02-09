use crate::net::{EthernetDevice, list_ethernet_devices};
use anyhow::Result;
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};
use tokio::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub kind: ToastKind,
    pub msg: String,
    pub until: Instant,
}

#[derive(Debug, Clone)]
pub struct CmdOutput {
    pub program: String,
    pub args: Vec<String>,
    pub used_sudo: bool,
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct App {
    pub running: bool,
    pub devices: Vec<EthernetDevice>,
    pub devices_state: TableState,
    pub last_error: Option<String>,
    pub last_action: Option<String>,
    pub toast: Option<Toast>,
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
            toast: None,
        })
    }

    pub async fn tick(&mut self) -> Result<()> {
        if let Some(t) = &self.toast
            && Instant::now() >= t.until
        {
            self.toast = None;
        }

        // Refresh state periodically so link/IP changes show up without restarting the TUI.
        match list_ethernet_devices() {
            Ok(devices) => {
                let selected = self.devices_state.selected();
                self.devices = devices;
                if self.devices.is_empty() {
                    self.devices_state.select(None);
                } else if let Some(i) = selected {
                    self.devices_state
                        .select(Some(i.min(self.devices.len() - 1)));
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

    pub fn set_toast(&mut self, kind: ToastKind, msg: impl Into<String>) {
        self.toast = Some(Toast {
            kind,
            msg: msg.into(),
            until: Instant::now() + Duration::from_millis(2500),
        });
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    pub async fn notify(&self, title: &str, body: &str) {
        // Best-effort desktop notification (Omarchy uses mako). Ignore failures.
        let _ = Command::new("notify-send")
            .arg(title)
            .arg(body)
            .arg("-t")
            .arg("2000")
            .output()
            .await;
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
        self.devices_state
            .selected()
            .and_then(|i| self.devices.get(i))
    }

    fn selected_iface(&self) -> Result<String> {
        self.selected_device()
            .map(|d| d.name.clone())
            .ok_or_else(|| std::io::Error::other("no interface selected").into())
    }

    async fn run_privileged_capture(&mut self, program: &str, args: &[&str]) -> Result<CmdOutput> {
        let mk = |used_sudo: bool, status: i32, stdout: Vec<u8>, stderr: Vec<u8>| CmdOutput {
            program: program.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            used_sudo,
            status,
            stdout: String::from_utf8_lossy(&stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&stderr).trim().to_string(),
        };

        // Try without sudo first (works if running as root or with capabilities/polkit).
        if let Ok(out) = Command::new(program).args(args).output().await {
            let code = out.status.code().unwrap_or(1);
            if out.status.success() {
                return Ok(mk(false, code, out.stdout, out.stderr));
            }

            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if stderr.contains("Operation not permitted")
                || stderr.contains("Permission denied")
                || out.status.code() == Some(1)
            {
                let sudo_out = Command::new("sudo")
                    .arg("-n")
                    .arg(program)
                    .args(args)
                    .output()
                    .await?;
                let code = sudo_out.status.code().unwrap_or(1);
                if sudo_out.status.success() {
                    return Ok(mk(true, code, sudo_out.stdout, sudo_out.stderr));
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

        Err(std::io::Error::other("failed to spawn command").into())
    }

    pub async fn renew_dhcp(&mut self) -> Result<CmdOutput> {
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
                return Ok(CmdOutput {
                    program: "networkctl".to_string(),
                    args: vec!["renew".to_string(), iface.clone()],
                    used_sudo: false,
                    status: out.status.code().unwrap_or(0),
                    stdout: String::from_utf8_lossy(&out.stdout).trim().to_string(),
                    stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
                });
            }
            let err = String::from_utf8_lossy(&out.stderr);
            if err.contains("Unknown") || err.contains("invalid") {
                let cap = self
                    .run_privileged_capture("networkctl", &["reconfigure", &iface])
                    .await?;
                self.last_action = Some(format!("Reconfigured {iface}"));
                return Ok(cap);
            }
        }

        let cap = self
            .run_privileged_capture("networkctl", &["renew", &iface])
            .await?;
        self.last_action = Some(format!("Renewed DHCP on {iface}"));
        Ok(cap)
    }
}

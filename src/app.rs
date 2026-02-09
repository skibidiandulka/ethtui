use crate::net::{EthernetDevice, list_ethernet_devices};
use anyhow::Result;
use ratatui::widgets::TableState;

pub struct App {
    pub running: bool,
    pub devices: Vec<EthernetDevice>,
    pub devices_state: TableState,
    pub last_error: Option<String>,
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
}


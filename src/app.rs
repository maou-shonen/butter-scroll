#[derive(Debug, Clone, Copy)]
pub enum AppCommand {
    ToggleEnabled,
    ReloadConfig,
    ToggleAutostart,
    Exit,
}

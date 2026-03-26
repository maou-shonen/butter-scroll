#[derive(Debug, Clone, Copy)]
pub enum AppCommand {
    ToggleEnabled,
    ToggleKeyboard,
    ReloadConfig,
    ToggleAutostart,
    Exit,
}

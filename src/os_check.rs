// Detects whether the OS will actually display a notification, independent
// of whether the underlying show() call reports success. On Windows, a
// disabled "Notifications" master toggle makes notify-rust's WinRT call
// return Ok(()) while Windows silently drops the toast - this exists to
// catch exactly that case and warn the operator instead of leaving them
// wondering why nothing ever appeared.

/// `Some(true)`/`Some(false)` when the platform's setting is known;
/// `None` when this platform isn't checked yet (assume enabled - never
/// warn on a platform we haven't verified a real signal for).
pub fn os_notifications_enabled() -> Option<bool> {
    #[cfg(target_os = "windows")]
    {
        windows::toast_enabled()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Interprets the raw `ToastEnabled` DWORD read from the registry.
/// Pulled out so the decision logic is testable without touching a real
/// registry: a missing key/value is `None` (unknown, don't warn) rather
/// than assumed disabled, since some Windows configurations may not have
/// written this value at all.
fn interpret_toast_enabled(raw: Option<u32>) -> Option<bool> {
    match raw {
        Some(0) => Some(false),
        Some(_) => Some(true),
        None => None,
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::interpret_toast_enabled;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    pub fn toast_enabled() -> Option<bool> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\PushNotifications")
            .ok()?;
        let raw: Option<u32> = key.get_value("ToastEnabled").ok();
        interpret_toast_enabled(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_means_disabled() {
        assert_eq!(interpret_toast_enabled(Some(0)), Some(false));
    }

    #[test]
    fn nonzero_means_enabled() {
        assert_eq!(interpret_toast_enabled(Some(1)), Some(true));
        assert_eq!(interpret_toast_enabled(Some(42)), Some(true));
    }

    #[test]
    fn missing_value_is_unknown_not_disabled() {
        assert_eq!(interpret_toast_enabled(None), None);
    }
}

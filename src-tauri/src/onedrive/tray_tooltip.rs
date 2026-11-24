use crate::models::SyncState;
use crate::onedrive::status_ui::{parse_status_text, ParsedStatusText};

#[derive(Debug, Clone)]
pub struct TrayTooltipEntry {
    pub account_title: String,
    pub status_text: String,
    pub state: SyncState,
    pub state_label: String,
    pub parsed: ParsedStatusText,
}

/// Parses a Windows 11 OneDrive taskbar button name / tooltip such as:
/// - "OneDrive - Personal\nBacked up and synced"
/// - "OneDrive - Personal\nUp to date"
/// - "OneDrive - Personal\nUploading 5 files (12.4 MB of 40.0 MB)"
/// - "OneDrive - Personal\nProcessing 1,245 changes"
/// - "OneDrive - Personal\nSync paused"
/// - "OneDrive - Contoso\nSync issues"
pub fn parse_onedrive_tooltip(raw: &str) -> Option<TrayTooltipEntry> {
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return None;
    }

    let first = lines[0];
    if !first.to_ascii_lowercase().contains("onedrive") {
        return None;
    }

    let account_title = first.to_string();
    let status_text = if lines.len() > 1 {
        lines[1..].join(" · ")
    } else {
        String::new()
    };

    let lower = status_text.to_ascii_lowercase();
    let (state, state_label) = classify_tooltip_status(&lower);
    let parsed = parse_status_text(&status_text, None);

    Some(TrayTooltipEntry {
        account_title,
        status_text: if status_text.is_empty() {
            state_label.clone()
        } else {
            status_text
        },
        state,
        state_label,
        parsed,
    })
}

fn classify_tooltip_status(lower: &str) -> (SyncState, String) {
    if lower.is_empty() {
        return (SyncState::InSync, "Up to date".to_string());
    }
    if lower.contains("not signed in")
        || lower.contains("sign in")
        || lower.contains("offline")
        || lower.contains("not connected")
        || lower.contains("tidak masuk")
    {
        return (SyncState::Offline, "Offline".to_string());
    }
    if lower.contains("paused") || lower.contains("dijeda") {
        return (SyncState::Paused, "Sync paused".to_string());
    }
    if lower.contains("error")
        || lower.contains("issue")
        || lower.contains("problem")
        || lower.contains("can't sync")
        || lower.contains("couldn't")
        || lower.contains("failed")
        || lower.contains("masalah")
    {
        return (SyncState::Error, "Sync error".to_string());
    }
    if lower.contains("attention")
        || lower.contains("almost full")
        || lower.contains("storage full")
        || lower.contains("quota")
        || lower.contains("warning")
    {
        return (SyncState::Warning, "Attention needed".to_string());
    }
    if lower.contains("uploading")
        || lower.contains("downloading")
        || lower.contains("syncing")
        || lower.contains("processing")
        || lower.contains("looking for")
        || lower.contains("signing in")
        || lower.contains("mengunggah")
        || lower.contains("mengunduh")
        || lower.contains("memproses")
        || lower.contains("menyinkronkan")
    {
        return (SyncState::Syncing, "Syncing".to_string());
    }
    if lower.contains("up to date")
        || lower.contains("backed up")
        || lower.contains("synced")
        || lower.contains("files are synced")
        || lower.contains("terbaru")
        || lower.contains("disinkronkan")
    {
        return (SyncState::InSync, "Up to date".to_string());
    }

    (SyncState::InSync, "Up to date".to_string())
}

#[cfg(windows)]
mod win_impl {
    use super::{parse_onedrive_tooltip, TrayTooltipEntry};
    use windows::core::{BSTR, VARIANT};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, TreeScope_Children,
        TreeScope_Descendants, UIA_ButtonControlTypeId, UIA_ClassNamePropertyId,
        UIA_ControlTypePropertyId,
    };

    /// Persistent UI Automation client querying Windows 11 taskbar and overflow tray buttons
    /// for live OneDrive status strings (works under both Medium and High integrity levels).
    #[derive(Default)]
    pub struct TrayTooltipReader {
        automation: Option<IUIAutomation>,
    }

    impl TrayTooltipReader {
        pub fn new() -> Self {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            let automation = unsafe {
                CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                    .ok()
            };
            Self { automation }
        }

        pub fn read_entries(&mut self) -> Vec<TrayTooltipEntry> {
            if self.automation.is_none() {
                self.automation = unsafe {
                    CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                        .ok()
                };
            }
            let Some(ref uia) = self.automation else {
                return Vec::new();
            };

            let Ok(root) = (unsafe { uia.GetRootElement() }) else {
                return Vec::new();
            };

            let mut results = Vec::new();
            for class_name in ["Shell_TrayWnd", "TopLevelWindowForOverflowXamlIsland"] {
                if let Some(container) = find_child_by_class(uia, &root, class_name) {
                    collect_onedrive_buttons(uia, &container, &mut results);
                }
            }
            results
        }
    }

    fn find_child_by_class(
        uia: &IUIAutomation,
        root: &IUIAutomationElement,
        class_name: &str,
    ) -> Option<IUIAutomationElement> {
        let var = VARIANT::from(BSTR::from(class_name));
        let cond = unsafe { uia.CreatePropertyCondition(UIA_ClassNamePropertyId, &var).ok()? };
        unsafe { root.FindFirst(TreeScope_Children, &cond).ok() }
    }

    fn collect_onedrive_buttons(
        uia: &IUIAutomation,
        container: &IUIAutomationElement,
        out: &mut Vec<TrayTooltipEntry>,
    ) {
        let var = VARIANT::from(UIA_ButtonControlTypeId.0);
        let Ok(cond) = (unsafe { uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &var) })
        else {
            return;
        };
        let Ok(elements) = (unsafe { container.FindAll(TreeScope_Descendants, &cond) }) else {
            return;
        };
        let Ok(len) = (unsafe { elements.Length() }) else {
            return;
        };

        for idx in 0..len {
            let Ok(el) = (unsafe { elements.GetElement(idx) }) else {
                continue;
            };
            let Ok(name_bstr) = (unsafe { el.CurrentName() }) else {
                continue;
            };
            let name = name_bstr.to_string();
            if name.to_ascii_lowercase().contains("onedrive") {
                if let Some(entry) = parse_onedrive_tooltip(&name) {
                    if !out
                        .iter()
                        .any(|e| e.account_title.eq_ignore_ascii_case(&entry.account_title))
                    {
                        out.push(entry);
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
pub use win_impl::TrayTooltipReader;

#[cfg(not(windows))]
#[derive(Default)]
pub struct TrayTooltipReader;

#[cfg(not(windows))]
impl TrayTooltipReader {
    pub fn new() -> Self {
        Self
    }

    pub fn read_entries(&mut self) -> Vec<TrayTooltipEntry> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_backed_up_and_synced_tooltip() {
        let entry = parse_onedrive_tooltip("OneDrive - Personal\r\nBacked up and synced").unwrap();
        assert_eq!(entry.account_title, "OneDrive - Personal");
        assert_eq!(entry.state, SyncState::InSync);
        assert_eq!(entry.state_label, "Up to date");
        assert_eq!(entry.status_text, "Backed up and synced");
    }

    #[test]
    fn parses_uploading_tooltip_with_bytes() {
        let entry =
            parse_onedrive_tooltip("OneDrive - Personal\nUploading 4 files (15.0 MB of 60.0 MB)")
                .unwrap();
        assert_eq!(entry.state, SyncState::Syncing);
        assert_eq!(entry.parsed.files_uploading, Some(4));
        assert!((entry.parsed.progress_percent.unwrap() - 25.0).abs() < 0.2);
    }

    #[test]
    fn parses_paused_and_error_tooltips() {
        let paused = parse_onedrive_tooltip("OneDrive - Work\nSync paused for 2 hours").unwrap();
        assert_eq!(paused.state, SyncState::Paused);

        let err = parse_onedrive_tooltip("OneDrive - Personal\nSync issues - 2 files").unwrap();
        assert_eq!(err.state, SyncState::Error);
    }
}

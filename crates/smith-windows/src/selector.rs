// crates/smith-windows/src/selector.rs
use uiautomation::core::{UIAutomation, UICondition, UIElement};
use uiautomation::types::{ControlType, PropertyConditionFlags, TreeScope, UIProperty};
use uiautomation::variants::Variant;

use smith_core::SmithError;

/// Builder-style selector for finding Windows UI elements.
///
/// Uses `uiautomation::Condition` combinators to build a query.
#[derive(Debug, Clone, Default)]
pub struct ElementSelector {
    pid: Option<u32>,
    name: Option<String>,
    automation_id: Option<String>,
    control_type: Option<String>,
    class_name: Option<String>,
}

impl ElementSelector {
    /// Creates a new `ElementSelector` with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filters by process ID.
    #[must_use]
    pub fn pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    /// Filters by element name (exact match).
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Filters by automation ID.
    #[must_use]
    pub fn automation_id(mut self, automation_id: impl Into<String>) -> Self {
        self.automation_id = Some(automation_id.into());
        self
    }

    /// Filters by control type name (e.g. "Button", "Edit", "Window").
    #[must_use]
    pub fn control_type(mut self, control_type: impl Into<String>) -> Self {
        self.control_type = Some(control_type.into());
        self
    }

    /// Filters by class name.
    #[must_use]
    pub fn class_name(mut self, class_name: impl Into<String>) -> Self {
        self.class_name = Some(class_name.into());
        self
    }

    /// Builds a `UICondition` from the set fields using an existing `UIAutomation` instance.
    ///
    /// This avoids creating a new COM initialization on each call.
    fn build_condition_with(&self, automation: &UIAutomation) -> Result<UICondition, SmithError> {
        let mut conditions: Vec<UICondition> = Vec::new();

        if let Some(ref name) = self.name {
            let cond = automation
                .create_property_condition(
                    UIProperty::Name,
                    Variant::from(name.as_str()),
                    Some(PropertyConditionFlags::None),
                )
                .map_err(|e| SmithError::PlatformError {
                    message: "Name property condition failed".into(),
                    source: Box::new(e),
                })?;
            conditions.push(cond);
        }

        if let Some(ref aid) = self.automation_id {
            let cond = automation
                .create_property_condition(
                    UIProperty::AutomationId,
                    Variant::from(aid.as_str()),
                    Some(PropertyConditionFlags::None),
                )
                .map_err(|e| SmithError::PlatformError {
                    message: "AutomationId property condition failed".into(),
                    source: Box::new(e),
                })?;
            conditions.push(cond);
        }

        if let Some(ref ct) = self.control_type
            && let Some(ct_value) = parse_control_type(ct)
        {
            let cond = automation
                .create_property_condition(
                    UIProperty::ControlType,
                    Variant::from(ct_value),
                    Some(PropertyConditionFlags::None),
                )
                .map_err(|e| SmithError::PlatformError {
                    message: "ControlType property condition failed".into(),
                    source: Box::new(e),
                })?;
            conditions.push(cond);
        }

        if let Some(ref cn) = self.class_name {
            let cond = automation
                .create_property_condition(
                    UIProperty::ClassName,
                    Variant::from(cn.as_str()),
                    Some(PropertyConditionFlags::None),
                )
                .map_err(|e| SmithError::PlatformError {
                    message: "ClassName property condition failed".into(),
                    source: Box::new(e),
                })?;
            conditions.push(cond);
        }

        if let Some(pid) = self.pid {
            let cond = automation
                .create_property_condition(
                    UIProperty::ProcessId,
                    Variant::from(pid.cast_signed()),
                    Some(PropertyConditionFlags::None),
                )
                .map_err(|e| SmithError::PlatformError {
                    message: "ProcessId property condition failed".into(),
                    source: Box::new(e),
                })?;
            conditions.push(cond);
        }

        if conditions.is_empty() {
            return automation
                .create_true_condition()
                .map_err(|e| SmithError::PlatformError {
                    message: "True condition creation failed".into(),
                    source: Box::new(e),
                });
        }

        // conditions.is_empty() checked above — guaranteed at least one element
        let mut iter = conditions.into_iter();
        // SAFETY: is_empty() checked above
        let first = iter.next().unwrap();
        iter.try_fold(first, |cond_acc, cond| {
            automation
                .create_and_condition(cond_acc, cond)
                .map_err(|e| SmithError::PlatformError {
                    message: "And condition creation failed".into(),
                    source: Box::new(e),
                })
        })
    }

    /// Finds the first matching element under `root`.
    ///
    /// # Errors
    ///
    /// Returns `SmithError::ElementNotFound` if no element matches.
    pub fn find_first(
        &self,
        root: &UIElement,
        automation: &UIAutomation,
    ) -> Result<UIElement, SmithError> {
        let condition = self.build_condition_with(automation)?;
        root.find_first(TreeScope::Descendants, &condition)
            .map_err(|_| SmithError::ElementNotFound)
    }

    /// Finds all matching elements under `root`.
    ///
    /// # Errors
    ///
    /// Returns `SmithError::PlatformError` if the UIA `find_all` call fails.
    pub fn find_all(
        &self,
        root: &UIElement,
        automation: &UIAutomation,
    ) -> Result<Vec<UIElement>, SmithError> {
        let condition = self.build_condition_with(automation)?;
        root.find_all(TreeScope::Descendants, &condition)
            .map_err(|e| SmithError::PlatformError {
                message: "Find all failed".into(),
                source: Box::new(e),
            })
    }

    /// Finds the first matching element starting from the desktop root.
    ///
    /// Uses a single `UIAutomation` instance for both the condition and root element
    /// to avoid redundant COM initialization.
    ///
    /// # Errors
    ///
    /// Returns `SmithError::ElementNotFound` if no element matches.
    pub fn find_from_desktop(&self) -> Result<UIElement, SmithError> {
        let automation = UIAutomation::new().map_err(|e| SmithError::PlatformError {
            message: "UIAutomation init failed".into(),
            source: Box::new(e),
        })?;
        let root = automation
            .get_root_element()
            .map_err(|e| SmithError::PlatformError {
                message: "Get root element failed".into(),
                source: Box::new(e),
            })?;
        self.find_first(&root, &automation)
    }
}

/// Parses a control type string into its numeric UIA identifier.
fn parse_control_type(s: &str) -> Option<i32> {
    match s.to_lowercase().as_str() {
        "button" => Some(ControlType::Button as i32),
        "calendar" => Some(ControlType::Calendar as i32),
        "checkbox" => Some(ControlType::CheckBox as i32),
        "combobox" => Some(ControlType::ComboBox as i32),
        "edit" | "text" => Some(ControlType::Edit as i32),
        "hyperlink" => Some(ControlType::Hyperlink as i32),
        "image" => Some(ControlType::Image as i32),
        "listitem" => Some(ControlType::ListItem as i32),
        "list" => Some(ControlType::List as i32),
        "menu" => Some(ControlType::Menu as i32),
        "menubar" => Some(ControlType::MenuBar as i32),
        "menuitem" => Some(ControlType::MenuItem as i32),
        "progressbar" => Some(ControlType::ProgressBar as i32),
        "radiobutton" => Some(ControlType::RadioButton as i32),
        "scrollbar" => Some(ControlType::ScrollBar as i32),
        "slider" => Some(ControlType::Slider as i32),
        "spinner" => Some(ControlType::Spinner as i32),
        "statusbar" => Some(ControlType::StatusBar as i32),
        "tab" => Some(ControlType::Tab as i32),
        "tabitem" => Some(ControlType::TabItem as i32),
        "toolbar" => Some(ControlType::ToolBar as i32),
        "tooltip" => Some(ControlType::ToolTip as i32),
        "tree" => Some(ControlType::Tree as i32),
        "treeitem" => Some(ControlType::TreeItem as i32),
        "custom" => Some(ControlType::Custom as i32),
        "group" => Some(ControlType::Group as i32),
        "thumb" => Some(ControlType::Thumb as i32),
        "datagrid" => Some(ControlType::DataGrid as i32),
        "dataitem" => Some(ControlType::DataItem as i32),
        "document" => Some(ControlType::Document as i32),
        "splitbutton" => Some(ControlType::SplitButton as i32),
        "window" => Some(ControlType::Window as i32),
        "pane" => Some(ControlType::Pane as i32),
        "header" => Some(ControlType::Header as i32),
        "headeritem" => Some(ControlType::HeaderItem as i32),
        "table" => Some(ControlType::Table as i32),
        "titlebar" => Some(ControlType::TitleBar as i32),
        "separator" => Some(ControlType::Separator as i32),
        "appbar" => Some(ControlType::AppBar as i32),
        _ => None,
    }
}

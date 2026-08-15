// crates/smith-windows/src/element.rs
use std::sync::Arc;
use uiautomation::UIElement;

/// Newtype that marks [`UIElement`] as `Send + Sync` for use in [`Arc`].
///
/// # Safety
/// `UIElement` internally contains `NonNull<c_void>` (a COM interface).
/// UI Automation objects are free-threaded — they can be safely shared
/// across threads. All mutations go through `spawn_blocking`.
#[derive(Debug)]
#[repr(transparent)]
struct SendUiElement(UIElement);

// SAFETY: UI Automation COM objects are free-threaded.
unsafe impl Send for SendUiElement {}
// SAFETY: UI Automation COM objects are free-threaded.
unsafe impl Sync for SendUiElement {}

/// Thread-safe wrapper over `UIElement`.
///
/// # Safety
///
/// `UIElement` internally contains `NonNull<c_void>` (a COM interface), which
/// does not implement `Send`. However, UI Automation objects are
/// free-threaded and can be safely transferred between threads.
/// All mutations (clicks, input) are performed via `spawn_blocking`,
/// where COM calls happen on a dedicated thread.
#[derive(Debug)]
pub struct SafeUIElement(Arc<SendUiElement>);

impl SafeUIElement {
    /// Creates a new thread-safe wrapper.
    #[must_use]
    pub fn new(element: UIElement) -> Self {
        Self(Arc::new(SendUiElement(element)))
    }

    /// Returns a reference to the inner element.
    #[must_use]
    pub fn inner(&self) -> &UIElement {
        &self.0.0
    }
}

// `SafeUIElement(Arc<SendUiElement>)` is automatically `Send + Sync`
// because `SendUiElement: Send + Sync`.

impl Clone for SafeUIElement {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

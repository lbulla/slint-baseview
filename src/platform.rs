use i_slint_core::{api::PlatformError, platform::Platform, window::WindowAdapter};
use std::{cell::RefCell, rc::Rc};

use crate::window_adapter::EmbeddedWindowAdapter;

// ---------- EmbeddedPlatform ---------- //

#[derive(Default)]
pub(crate) struct EmbeddedPlatform {}

impl EmbeddedPlatform {
    thread_local! {
        pub(crate) static WINDOW_ADAPTER_INNER: RefCell<Option<Rc<EmbeddedWindowAdapter>>> = Default::default();
    }
}

impl Platform for EmbeddedPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Self::WINDOW_ADAPTER_INNER.with_borrow_mut(|a| match a.take() {
            Some(a) => Ok(a as _),
            None => Err(PlatformError::Other("No `WINDOW_ADAPTER_INNER`".into())),
        })
    }
}

use i_slint_core::{api::PlatformError, platform::Platform, window::WindowAdapter};
use std::{cell::RefCell, rc::Rc};

use crate::SbWindowAdapter;

// ---------- SbPlatform ---------- //

#[derive(Default)]
pub(crate) struct SbPlatform {}

impl SbPlatform {
    thread_local! {
        pub(crate) static WINDOW_ADAPTER: RefCell<Option<Rc<SbWindowAdapter>>> = Default::default();
    }
}

// TODO: impl complete trait.
impl Platform for SbPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Self::WINDOW_ADAPTER.with_borrow_mut(|a| match a.take() {
            Some(a) => Ok(a as _),
            None => Err(PlatformError::Other("No `WINDOW_ADAPTER_INNER`".into())),
        })
    }
}

use i_slint_core::{items::MenuEntry, menus::MenuVTable};
use muda::ContextMenu;
use strum::{AsRefStr, EnumString};
use vtable::{VRc, VRef};

// ---------- MudaAdapter ---------- //

// Taken from https://github.com/slint-ui/slint/blob/master/internal/backends/winit/muda.rs.

pub(crate) const ID_SEP: char = ',';

#[derive(Clone, Copy, PartialEq, AsRefStr, EnumString)]
pub(crate) enum MudaType {
    Menubar,
    Context,
}

pub(crate) struct MudaAdapter {
    table: VRc<MenuVTable>,
    entries: Vec<MenuEntry>,
    menu: muda::Menu,
}

impl MudaAdapter {
    pub(crate) fn setup(table: VRc<MenuVTable>, muda_type: MudaType) -> Self {
        fn generate_menu_entry(
            menu: VRef<'_, MenuVTable>,
            entry: &MenuEntry,
            depth: usize,
            entries: &mut Vec<MenuEntry>,
            muda_type: MudaType,
        ) -> Box<dyn muda::IsMenuItem> {
            let id = format!(
                "{}{ID_SEP}{}",
                muda_type.as_ref(),
                entries.len().to_string()
            );
            entries.push(entry.clone());

            if entry.is_separator {
                Box::new(muda::PredefinedMenuItem::separator())
            } else if !entry.has_sub_menu {
                // the top level always has a sub menu regardless of entry.has_sub_menu
                if entry.checkable {
                    Box::new(muda::CheckMenuItem::with_id(
                        &id,
                        &entry.title,
                        entry.enabled,
                        entry.checked,
                        None,
                    ))
                } else if let Some(rgba) = entry.icon.to_rgba8() {
                    let icon = muda::Icon::from_rgba(
                        rgba.as_bytes().to_vec(),
                        rgba.width(),
                        rgba.height(),
                    )
                    .ok();
                    Box::new(muda::IconMenuItem::with_id(
                        &id,
                        &entry.title,
                        entry.enabled,
                        icon,
                        None,
                    ))
                } else {
                    Box::new(muda::MenuItem::with_id(
                        &id,
                        &entry.title,
                        entry.enabled,
                        None,
                    ))
                }
            } else {
                let sub_menu = muda::Submenu::with_id(&id, &entry.title, entry.enabled);
                if depth < 15 {
                    let mut sub_entries = Default::default();
                    menu.sub_menu(Some(entry), &mut sub_entries);
                    for e in sub_entries {
                        sub_menu
                            .append(&*generate_menu_entry(
                                menu,
                                &e,
                                depth + 1,
                                entries,
                                muda_type,
                            ))
                            .unwrap();
                    }
                } else {
                    // infinite menu depth is possible, but we want to limit the amount of item passed to muda
                    sub_menu
                        .append(&muda::MenuItem::with_id(
                            &id,
                            "<Error: Menu Depth limit reached>",
                            false,
                            None,
                        ))
                        .unwrap();
                }
                Box::new(sub_menu)
            }
        }

        let mut entries = vec![];
        let menu = muda::Menu::new();

        #[cfg(target_os = "macos")]
        {
            if muda_type == MudaType::Menubar {
                menu.init_for_nsapp();
                Self::create_default_app_menu(&menu);
            }
        }

        let mut menu_entries = Default::default();
        VRc::borrow(&table).sub_menu(None, &mut menu_entries);

        for menu_entry in menu_entries {
            menu.append(&*generate_menu_entry(
                VRc::borrow(&table),
                &menu_entry,
                0,
                &mut entries,
                muda_type,
            ))
            .unwrap();
        }

        Self {
            table,
            entries,
            menu,
        }
    }

    pub(crate) fn show_context_menu(
        table: VRc<MenuVTable>,
        window_handle: raw_window_handle::RawWindowHandle,
        position: i_slint_core::api::LogicalPosition,
        scale_factor: f64,
    ) -> Option<Self> {
        match window_handle {
            #[cfg(target_os = "macos")]
            raw_window_handle::RawWindowHandle::AppKit(handle) => {
                let this = Self::setup(table, MudaType::Context);
                let view: &objc2_app_kit::NSView = unsafe { &*handle.ns_view.as_ptr().cast() };
                let view_rect = view.frame();
                unsafe {
                    this.menu.show_context_menu_for_nsview(
                        handle.ns_view.as_ptr(),
                        Some(
                            muda::dpi::LogicalPosition::new(
                                scale_factor * position.x as f64,
                                view_rect.size.height - scale_factor * position.y as f64,
                            )
                            .into(),
                        ),
                    );
                }
                Some(this)
            }
            #[cfg(target_os = "windows")]
            raw_window_handle::RawWindowHandle::Win32(handle) => {
                let this = Self::setup(table, MudaType::Context);
                unsafe {
                    this.menu.show_context_menu_for_hwnd(
                        handle.ns_view.as_ptr(),
                        Some(
                            muda::dpi::LogicalPosition::new(
                                scale_factor * position.x as f64,
                                scale_factor * position.y as f64,
                            )
                            .into(),
                        ),
                    );
                }
                Some(this)
            }
            // TODO: Linux.
            _ => None,
        }
    }

    pub(crate) fn invoke(&self, index: usize) {
        let entry = &self.entries[index];
        VRc::borrow(&self.table).activate(entry);
    }

    #[cfg(target_os = "macos")]
    fn create_default_app_menu(menu: &muda::Menu) {
        let app_menu = muda::Submenu::new("App", true);
        if let Err(err) = menu.append(&app_menu).and_then(|_| {
            app_menu.append_items(&[
                &muda::PredefinedMenuItem::about(None, None),
                &muda::PredefinedMenuItem::separator(),
                &muda::PredefinedMenuItem::services(None),
                &muda::PredefinedMenuItem::separator(),
                &muda::PredefinedMenuItem::hide(None),
                &muda::PredefinedMenuItem::hide_others(None),
                &muda::PredefinedMenuItem::show_all(None),
                &muda::PredefinedMenuItem::separator(),
                &muda::PredefinedMenuItem::quit(None),
            ])
        }) {
            eprintln!("Could not create the default menu: {err}");
        }
    }
}

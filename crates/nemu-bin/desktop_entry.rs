use std::sync::OnceLock;

use freedesktop_desktop_entry::DesktopEntry;
use gtk4::{
    gio,
    glib::{self, object::Cast, subclass::types::ObjectSubclassIsExt as _},
};

mod imp {
    use glib::subclass::prelude::*;
    use gtk4::{gio, glib};
    use std::{
        cell::{Cell, RefCell},
        collections::BTreeMap,
        path::PathBuf,
    };

    use crate::desktop_entry::DesktopEntryActionKind;

    pub struct DesktopEntryObject {
        pub entry: Cell<&'static freedesktop_desktop_entry::DesktopEntry>,
        pub score: Cell<i64>,
        pub actions: RefCell<Option<gio::ListModel>>,
    }

    fn default_entry() -> &'static freedesktop_desktop_entry::DesktopEntry {
        static DEFAULT_ENTRY: freedesktop_desktop_entry::DesktopEntry =
            freedesktop_desktop_entry::DesktopEntry {
                appid: String::new(),
                groups: freedesktop_desktop_entry::Groups(BTreeMap::new()),
                path: PathBuf::new(),
                ubuntu_gettext_domain: None,
            };
        &DEFAULT_ENTRY
    }

    impl Default for DesktopEntryObject {
        fn default() -> Self {
            Self {
                entry: Cell::new(default_entry()),
                score: Cell::new(0),
                actions: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DesktopEntryObject {
        const NAME: &'static str = "NemuDesktopEntryObject";
        type Type = super::DesktopEntryObject;
    }
    impl ObjectImpl for DesktopEntryObject {}

    pub struct DesktopEntryActionObject {
        pub entry: Cell<&'static freedesktop_desktop_entry::DesktopEntry>,
        pub action: Cell<DesktopEntryActionKind>,
    }

    impl Default for DesktopEntryActionObject {
        fn default() -> Self {
            Self {
                entry: Cell::new(default_entry()),
                action: Cell::new(DesktopEntryActionKind::Launch),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DesktopEntryActionObject {
        const NAME: &'static str = "NemuDesktopEntryActionObject";
        type Type = super::DesktopEntryActionObject;
    }
    impl ObjectImpl for DesktopEntryActionObject {}
}

glib::wrapper! {
    pub struct DesktopEntryObject(ObjectSubclass<imp::DesktopEntryObject>);
}
glib::wrapper! {
    pub struct DesktopEntryActionObject(ObjectSubclass<imp::DesktopEntryActionObject>);
}

impl DesktopEntryObject {
    pub fn entry(&self) -> &'static freedesktop_desktop_entry::DesktopEntry {
        self.imp().entry.get()
    }

    pub fn score(&self) -> i64 {
        self.imp().score.get()
    }

    pub fn set_score(&self, score: i64) {
        self.imp().score.set(score)
    }

    pub fn actions(&self) -> gio::ListModel {
        let imp = self.imp();
        let entry = imp.entry.get();
        let mut actions = imp.actions.borrow_mut();
        let actions = match &mut *actions {
            Some(actions) => actions.clone(),
            None => {
                let actions_model = gio::ListStore::new::<DesktopEntryActionObject>();

                if entry.dbus_activatable() {
                    actions_model.append(&DesktopEntryActionObject::new(
                        self.clone(),
                        DesktopEntryActionKind::DBusActivate,
                    ));
                } else {
                    actions_model.append(&DesktopEntryActionObject::new(
                        self.clone(),
                        DesktopEntryActionKind::Launch,
                    ));
                }

                if let Some(entry_actions) = entry.actions() {
                    for action in entry_actions {
                        if let Some(action_exec) = entry.action_exec(action) {
                            actions_model.append(&DesktopEntryActionObject::new(
                                self.clone(),
                                DesktopEntryActionKind::Named(action, action_exec),
                            ));
                        }
                    }
                }

                *actions = Some(actions_model.clone().upcast());
                actions_model.upcast()
            }
        };
        actions
    }
}

impl DesktopEntryActionObject {
    fn new(entry: DesktopEntryObject, kind: DesktopEntryActionKind) -> Self {
        let action: Self = glib::Object::new();
        let imp = action.imp();
        imp.entry.set(entry.entry());
        imp.action.set(kind);
        action
    }

    pub fn entry(&self) -> &'static freedesktop_desktop_entry::DesktopEntry {
        self.imp().entry.get()
    }

    pub fn kind(&self) -> DesktopEntryActionKind {
        self.imp().action.get()
    }
}

struct AppCache {
    all: gtk4::gio::ListModel,
}

fn cached_desktop_entries() -> &'static [DesktopEntry] {
    static SYSTEM_APPS: OnceLock<&'static [DesktopEntry]> = OnceLock::new();

    fn leak_freedesktop_apps() -> &'static [DesktopEntry] {
        let locales = freedesktop_desktop_entry::get_languages_from_env();
        let mut apps = freedesktop_desktop_entry::desktop_entries(&locales);
        let mut i = 0;
        while i < apps.len() {
            // TODO: Support launching terminal apps
            if apps[i].hidden() || apps[i].no_display() || apps[i].terminal() {
                apps.swap_remove(i);
            } else {
                i += 1;
            }
        }
        Vec::leak(apps)
    }

    SYSTEM_APPS.get_or_init(leak_freedesktop_apps)
}

impl AppCache {
    fn init() -> Self {
        let all = gtk4::gio::ListStore::new::<DesktopEntryObject>();
        for entry in cached_desktop_entries() {
            let item: DesktopEntryObject = glib::Object::new();
            item.imp().entry.set(entry);
            all.append(&item);
        }
        Self { all: all.upcast() }
    }
}

thread_local! {
    static CACHE: AppCache = AppCache::init();
}

pub fn get_all_apps() -> gtk4::gio::ListModel {
    CACHE.with(|c| c.all.clone())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DesktopEntryActionKind {
    Launch,
    DBusActivate,
    Named(&'static str, &'static str),
}

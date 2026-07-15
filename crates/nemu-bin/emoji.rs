use std::collections::HashMap;

use gtk4::glib::{self, object::Cast, subclass::types::ObjectSubclassIsExt};

mod imp {
    use glib::subclass::prelude::*;
    use gtk4::glib;
    use std::cell::Cell;

    pub struct EmojiObject {
        pub emoji: Cell<&'static emojis::Emoji>,
        pub score: Cell<i64>,
    }

    impl Default for EmojiObject {
        fn default() -> Self {
            Self {
                emoji: Cell::new(emojis::get("😀").unwrap()),
                score: Cell::new(0),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EmojiObject {
        const NAME: &'static str = "NemuEmojiObject";
        type Type = super::EmojiObject;
    }
    impl ObjectImpl for EmojiObject {}
}

glib::wrapper! {
    /// GObject representation of an Emoji. These are lazily allocated for all emojis when
    /// the emoji picker is opened.
    pub struct EmojiObject(ObjectSubclass<imp::EmojiObject>);
}

impl EmojiObject {
    fn new(emoji: &'static emojis::Emoji) -> Self {
        let obj: Self = glib::Object::new();
        obj.imp().emoji.set(emoji);
        obj
    }

    pub fn emoji(&self) -> &'static emojis::Emoji {
        self.imp().emoji.get()
    }
    pub fn score(&self) -> i64 {
        self.imp().score.get()
    }
    pub fn set_score(&self, score: i64) {
        self.imp().score.set(score)
    }
}

struct EmojiCaches {
    groups: Vec<(emojis::Group, gtk4::gio::ListModel)>,
    all: gtk4::gio::ListModel,
}

impl EmojiCaches {
    fn init() -> Self {
        let mut groups = HashMap::<emojis::Group, gtk4::gio::ListStore>::default();
        let all = gtk4::gio::ListStore::new::<EmojiObject>();

        for emoji in emojis::iter() {
            let emoji_object = EmojiObject::new(emoji);
            all.append(&emoji_object);
            groups
                .entry(emoji.group())
                .or_insert_with(gtk4::gio::ListStore::new::<EmojiObject>)
                .append(&emoji_object);
        }

        // Preserve the order
        let groups = emojis::Group::iter()
            .map(|group| (group, groups.remove(&group).unwrap().upcast()))
            .collect();

        EmojiCaches {
            groups,
            all: all.upcast(),
        }
    }
}

thread_local! {
    static CACHES: EmojiCaches = EmojiCaches::init();
}

pub fn get_all_emojis() -> gtk4::gio::ListModel {
    CACHES.with(|c| c.all.clone())
}

pub fn get_groups() -> Vec<(emojis::Group, gtk4::gio::ListModel)> {
    CACHES.with(|c| c.groups.clone())
}

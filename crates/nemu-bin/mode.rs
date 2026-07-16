use std::rc::Rc;

use gtk4::glib::GString;
use relm4::{Sender, prelude::*};

use crate::AppMsg;

/// Lazily initialized mode (App launcher, Emoji picker, etc.)
pub trait Mode {
    fn widget(&self) -> &gtk::Widget;
    fn sender(&self) -> &Sender<ModeMsg>;
}

/// Initializer for a particular mode.
pub trait ModeFactory {
    /// Name of this mode's view in the parent container. Must be unique among other modes.
    /// Used to (re)activate this mode while keeping other previously initialized modes intact.
    fn name(&self) -> &'static str;

    /// Initialize the mode. Can potentially be expensive, like listing all apps on the system,
    /// or categorizing emojis.
    fn create(&self, sender: Sender<AppMsg>, initial_query_string: &str) -> Rc<dyn Mode>;

    /// Called in the default launcher mode (no subcommand argument on the CLI) to determine
    /// if this
    fn should_switch_to_this_mode(&self, query_string: &str) -> bool;
}

#[derive(Debug)]
pub enum ModeMsg {
    Activate,
    Deactivate,
    ActivateCurrent,
    SetQueryString(GString),
}

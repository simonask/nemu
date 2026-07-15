use std::rc::Rc;

use gtk4::prelude::*;
use relm4::prelude::*;

use crate::mode::{Mode, ModeFactory, ModeMsg};

const HELP_MARKUP: &str = r#"<big><b>Help</b></big>

Start typing the name of an app to view matches, then select an app using arrow keys or the mouse to launch it.

<b>Modes:</b>

  🥰 If the search term starts with <tt>:</tt>, the Emoji picker mode is activated. Can also be activated directly by running <tt>nemu emoji</tt>.

  🤔 If the search term starts with <tt>=</tt>, the rest will be interpreted as a mathematical expression.

  ❓ If the search term starts with <tt>?</tt>, this help text is displayed. You already did that. 💖
  "#;

#[derive(Default)]
pub struct HelpModeFactory;

impl ModeFactory for HelpModeFactory {
    fn name(&self) -> &'static str {
        "help"
    }

    fn create(
        &self,
        _sender: relm4::Sender<crate::AppMsg>,
        _initial_query_string: &str,
    ) -> Rc<dyn crate::mode::Mode> {
        let controller = HelpModel::builder().launch(()).detach();
        Rc::new(HelpMode { controller })
    }

    fn should_switch_to_this_mode(&self, query_string: &str) -> bool {
        query_string.starts_with('?')
    }
}

struct HelpMode {
    controller: Controller<HelpModel>,
}

impl Mode for HelpMode {
    fn widget(&self) -> &gtk4::Widget {
        self.controller.widget().upcast_ref()
    }

    fn sender(&self) -> &relm4::Sender<ModeMsg> {
        self.controller.sender()
    }
}

struct HelpModel;

#[relm4::component]
impl SimpleComponent for HelpModel {
    type Init = ();
    type Input = ModeMsg;
    type Output = ();

    view! {
        gtk::Label {
            add_css_class: "nemu-help",
            add_css_class: "nemu-card",
            set_markup: HELP_MARKUP,
            set_wrap: true,
            set_use_markup: true,
            set_xalign: 0.0
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        ComponentParts {
            model: HelpModel,
            widgets,
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {}
}

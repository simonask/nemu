use std::rc::Rc;

use gtk4::prelude::*;
use relm4::prelude::*;

use crate::mode::{Mode, ModeFactory, ModeMsg};

#[derive(Default)]
pub struct CalculatorModeFactory;

impl ModeFactory for CalculatorModeFactory {
    fn name(&self) -> &'static str {
        "calc"
    }

    fn create(
        &self,
        _sender: relm4::Sender<crate::AppMsg>,
        _initial_query_string: &str,
    ) -> Rc<dyn crate::mode::Mode> {
        let controller = CalcModel::builder().launch(()).detach();
        Rc::new(CalcMode { controller })
    }

    fn should_switch_to_this_mode(&self, query_string: &str) -> bool {
        query_string.starts_with('=')
    }
}

struct CalcMode {
    controller: Controller<CalcModel>,
}

impl Mode for CalcMode {
    fn widget(&self) -> &gtk4::Widget {
        self.controller.widget().upcast_ref()
    }

    fn sender(&self) -> &relm4::Sender<ModeMsg> {
        self.controller.sender()
    }
}

struct CalcModel;

#[relm4::component]
impl SimpleComponent for CalcModel {
    type Init = ();
    type Input = ModeMsg;
    type Output = ();

    view! {
        gtk::Label {
            add_css_class: "nemu-help",
            add_css_class: "nemu-card",
            set_text: "Calculator"
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        ComponentParts {
            model: CalcModel,
            widgets,
        }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {}
}

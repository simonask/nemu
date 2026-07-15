use std::{cell::Cell, io::BufRead as _, path::PathBuf, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use gtk4::{glib, prelude::*};
use relm4::prelude::*;

use crate::{
    AppMsg,
    mode::{Mode, ModeFactory, ModeMsg},
};

#[derive(clap::Parser, Clone, PartialEq)]
pub struct DmenuArgs {
    /// Delimit arguments by the NUL character (zero) instead of newlines.
    #[clap(long, short = '0')]
    pub nul_delimiter: bool,
    /// Read strings from file instead of stdin.
    #[clap(long, short)]
    pub file: Option<PathBuf>,
    /// Split each input line by the TAB character and choose the n'th.
    #[clap(long)]
    pub with_nth: Option<usize>,
}

impl DmenuArgs {
    pub fn read_strings(&self) -> Result<Vec<String>, std::io::Error> {
        if let Some(path) = self.file.as_deref() {
            self.read_strings_from_input(std::fs::File::open(path)?)
        } else {
            self.read_strings_from_input(std::io::stdin())
        }
    }

    fn read_strings_from_input<R: std::io::Read>(
        &self,
        mut reader: R,
    ) -> Result<Vec<String>, std::io::Error> {
        let delimited: Vec<String> = if self.nul_delimiter {
            let mut buf = String::new();
            reader.read_to_string(&mut buf)?;
            buf.split('\0').map(str::to_owned).collect()
        } else {
            std::io::BufReader::new(reader)
                .lines()
                .collect::<Result<_, _>>()?
        };

        Ok(if let Some(with_nth) = self.with_nth {
            delimited
                .iter()
                .map(|line| {
                    if let Some(nth) = line.split('\t').nth(with_nth) {
                        nth.to_owned()
                    } else {
                        line.clone()
                    }
                })
                .collect()
        } else {
            delimited
        })
    }
}

pub struct DmenuModeFactory(pub Vec<String>);

impl ModeFactory for DmenuModeFactory {
    fn name(&self) -> &'static str {
        "dmenu"
    }

    fn create(&self, sender: relm4::Sender<crate::AppMsg>, _: &str) -> Rc<dyn crate::mode::Mode> {
        let controller = DmenuModel::builder()
            .launch(self.0.clone())
            .forward(&sender, std::convert::identity);
        Rc::new(DmenuMode { controller })
    }

    fn should_switch_to_this_mode(&self, _query_string: &str) -> bool {
        // dmenu mode can only be initialized from the command line
        false
    }
}

struct DmenuMode {
    controller: Controller<DmenuModel>,
}

impl Mode for DmenuMode {
    fn widget(&self) -> &gtk4::Widget {
        self.controller.widget().upcast_ref()
    }

    fn sender(&self) -> &relm4::Sender<ModeMsg> {
        self.controller.sender()
    }
}

struct DmenuModel {
    list_view: gtk::ListView,
    selection: gtk::SingleSelection,
    store: gtk::gio::ListStore,
    filter: gtk::CustomFilter,
    sorter: gtk::CustomSorter,
    matcher: fuzzy_matcher::skim::SkimMatcherV2,
}

struct DmenuItem {
    text: String,
    score: Cell<i64>,
}

#[relm4::component]
impl SimpleComponent for DmenuModel {
    type Init = Vec<String>;
    type Input = ModeMsg;
    type Output = AppMsg;

    view! {
        gtk::Box {
            add_css_class: "nemu-dmenu",
            add_css_class: "nemu-card",

            gtk::ScrolledWindow {
                #[local_ref]
                list_view -> gtk::ListView {
                    add_css_class: "nemu-results",
                    add_css_class: "flat",
                    set_orientation: gtk::Orientation::Vertical,
                    // set_spacing: 10,
                    set_hexpand: true,
                    set_tab_behavior: gtk::ListTabBehavior::Item,
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for s in init {
            store.append(&glib::BoxedAnyObject::new(DmenuItem {
                text: s,
                score: Cell::new(1),
            }))
        }
        let filter = gtk::CustomFilter::new(|obj| {
            obj.downcast_ref::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<DmenuItem>()
                .score
                .get()
                > 0
        });
        let sorter = gtk::CustomSorter::new(|a, b| {
            let a = a
                .downcast_ref::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<DmenuItem>()
                .score
                .get();
            let b = b
                .downcast_ref::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<DmenuItem>()
                .score
                .get();
            b.cmp(&a).into()
        });

        let filtered = gtk::FilterListModel::new(Some(store.clone()), Some(filter.clone()));
        let sorted = gtk::SortListModel::new(Some(filtered), Some(sorter.clone()));

        let selection = gtk::SingleSelection::new(Some(sorted));
        selection.set_autoselect(true);
        let list_view = gtk::ListView::default();
        let model = DmenuModel {
            list_view: list_view.clone(),
            store,
            filter,
            sorter,
            selection: selection.clone(),
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default()
                .use_cache(true)
                .ignore_case(),
        };
        let widgets = view_output!();

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::builder()
                .css_classes(["nemu-dmenu-item"])
                .halign(gtk::Align::Start)
                .hexpand(true)
                .build();
            list_item
                .downcast_ref::<gtk::ListItem>()
                .unwrap()
                .set_child(Some(&label));
        });
        factory.connect_bind(move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = item.child().and_downcast::<gtk::Label>().unwrap();
            let boxed = item.item().and_downcast::<glib::BoxedAnyObject>().unwrap();
            label.set_label(&boxed.borrow::<DmenuItem>().text);
        });
        widgets.list_view.set_factory(Some(&factory));
        list_view.set_model(Some(&selection));
        let s = sender.clone();
        list_view.connect_activate(move |list_view, position| {
            let item = list_view
                .model()
                .unwrap()
                .item(position)
                .and_downcast::<glib::BoxedAnyObject>()
                .unwrap();
            s.output_sender()
                .emit(AppMsg::TextOutput(item.borrow::<DmenuItem>().text.clone()));
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            ModeMsg::ActivateCurrent => {
                let pos = self.selection.selected();
                if pos != gtk::INVALID_LIST_POSITION {
                    self.list_view.emit_by_name::<()>("activate", &[&pos]);
                }
            }
            ModeMsg::SetQueryString(query_string) => {
                if query_string.is_empty() {
                    for i in 0..self.store.n_items() {
                        let obj = self.store.item(i).unwrap();
                        let item = obj
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .unwrap()
                            .borrow::<DmenuItem>();
                        item.score.set(1);
                    }
                } else {
                    for i in 0..self.store.n_items() {
                        let obj = self.store.item(i).unwrap();
                        let item = obj
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .unwrap()
                            .borrow::<DmenuItem>();
                        let score = self.matcher.fuzzy_match(&item.text, &query_string);
                        item.score.set(score.unwrap_or(-1));
                    }
                }
                self.filter.changed(gtk::FilterChange::Different);
                self.sorter.changed(gtk::SorterChange::Different);
                self.selection.set_selected(0);
            }
            _ => (),
        }
    }
}

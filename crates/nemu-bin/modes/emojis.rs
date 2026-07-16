use std::rc::Rc;

use fuzzy_matcher::FuzzyMatcher;
use gtk4::prelude::*;
use relm4::prelude::*;

use crate::{
    AppMsg,
    emoji::EmojiObject,
    mode::{Mode, ModeFactory, ModeMsg},
};

#[derive(clap::Parser, Clone, Copy)]
pub struct EmojiArgs {
    /// Show a notification when an Emoji was copied to the clipboard.
    #[clap(long, default_value = "false")]
    pub notify: bool,
}

pub struct EmojiPickerFactory(pub EmojiArgs);

impl ModeFactory for EmojiPickerFactory {
    fn name(&self) -> &'static str {
        "emojis"
    }

    fn create(
        &self,
        sender: relm4::Sender<crate::AppMsg>,
        initial_query_string: &str,
    ) -> Rc<dyn crate::mode::Mode> {
        let controller = EmojisModel::builder()
            .launch((initial_query_string.to_owned(), self.0))
            .forward(&sender, std::convert::identity);
        Rc::new(EmojiPickerMode { controller })
    }

    fn should_switch_to_this_mode(&self, query_string: &str) -> bool {
        query_string.starts_with(':')
    }
}

struct EmojiPickerMode {
    controller: Controller<EmojisModel>,
}

impl Mode for EmojiPickerMode {
    fn widget(&self) -> &gtk4::Widget {
        self.controller.widget().upcast_ref()
    }

    fn sender(&self) -> &relm4::Sender<ModeMsg> {
        self.controller.sender()
    }
}

struct EmojisModel {
    has_search_string: bool,
    picker: Controller<EmojiPickerModel>,
    search: Controller<EmojiSearchModel>,
}

fn get_emoji_group_tooltip(group: emojis::Group) -> &'static str {
    match group {
        emojis::Group::SmileysAndEmotion => "Smileys",
        emojis::Group::PeopleAndBody => "People & Body",
        emojis::Group::AnimalsAndNature => "Animals & Nature",
        emojis::Group::FoodAndDrink => "Food & Drink",
        emojis::Group::TravelAndPlaces => "Travel & Places",
        emojis::Group::Activities => "Activities",
        emojis::Group::Objects => "Objects",
        emojis::Group::Symbols => "Symbols",
        emojis::Group::Flags => "Flags",
    }
}

#[relm4::component]
impl SimpleComponent for EmojisModel {
    type Init = (String, EmojiArgs);
    type Input = ModeMsg;
    type Output = AppMsg;

    view! {
        #[name = "emoji_stack"]
        gtk::Stack {
            add_css_class: "nemu-emojis",

            add_named: (model.picker.widget(), Some("picker")),
            add_named: (model.search.widget(), Some("search")),
            #[watch]
            set_visible_child_name: if model.has_search_string { "search" } else { "picker"},
        }
    }

    fn init(
        (search_string, args): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, grid_item| {
            let label = gtk::Label::builder()
                .css_classes(["nemu-emoji"])
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .build();
            grid_item
                .downcast_ref::<gtk::ListItem>()
                .unwrap()
                .set_child(Some(&label));
        });
        factory.connect_bind(move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = item.child().and_downcast::<gtk::Label>().unwrap();
            let emoji = item.item().and_downcast::<EmojiObject>().unwrap();
            label.set_label(emoji.emoji().as_str());
        });

        let has_search_string = !search_string.is_empty();
        let picker = EmojiPickerModel::builder()
            .launch((factory.clone(), args))
            .forward(sender.output_sender(), std::convert::identity);
        let search = EmojiSearchModel::builder()
            .launch((factory, args))
            .forward(sender.output_sender(), std::convert::identity);

        let model = EmojisModel {
            has_search_string,
            picker,
            search,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            ModeMsg::SetQueryString(gstring) => {
                if gstring.is_empty() || gstring == ":" {
                    self.has_search_string = false;
                } else {
                    self.has_search_string = true;
                }
                self.search.sender().emit(ModeMsg::SetQueryString(gstring));
            }
            _ => (),
        }
    }
}

struct EmojiPickerModel;

#[relm4::component]
impl SimpleComponent for EmojiPickerModel {
    type Init = (gtk::SignalListItemFactory, EmojiArgs);
    type Input = ModeMsg;
    type Output = AppMsg;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,
            add_css_class: "nemu-card",
            add_css_class: "nemu-emoji-picker",

            #[name(switcher)]
            gtk::StackSwitcher {
                add_css_class: "nemu-emoji-tabs",
            },

            #[name(stack)]
            gtk::Stack {
                set_vexpand: true,
            }
        }
    }

    fn init(
        (factory, args): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = EmojiPickerModel;
        let widgets = view_output!();

        for (group, emoji_list) in crate::emoji::get_groups() {
            let grid_view = gtk::GridView::builder()
                .min_columns(10)
                .max_columns(30)
                .valign(gtk::Align::Start)
                .vexpand(false)
                .single_click_activate(true)
                .tab_behavior(gtk4::ListTabBehavior::Item)
                .build();

            let selection_model = gtk::SingleSelection::new(Some(emoji_list));
            selection_model.set_autoselect(true);
            grid_view.set_model(Some(&selection_model));
            grid_view.set_factory(Some(&factory));

            let s = sender.clone();
            grid_view.connect_activate(move |grid_view, position| {
                let item = grid_view
                    .model()
                    .unwrap()
                    .item(position)
                    .and_downcast::<EmojiObject>()
                    .unwrap();
                s.output_sender().emit(AppMsg::TextOutput {
                    text: item.emoji().as_str().to_owned(),
                    notify: args.notify,
                });
            });

            let scroller = gtk::ScrolledWindow::builder()
                .child(&grid_view)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(false)
                .build();

            // representative emoji as the *tab label* (StackSwitcher renders the title text)
            let tab = group.emojis().next().unwrap().as_str();
            widgets
                .stack
                .add_titled(&scroller, Some(get_emoji_group_tooltip(group)), tab);
        }

        widgets.switcher.set_stack(Some(&widgets.stack));

        // Set the tooltips for the switcher buttons.
        let mut child = widgets.switcher.first_child();
        let mut groups = emojis::Group::iter();
        while let (Some(button), Some(group)) = (child, groups.next()) {
            button.set_tooltip_text(Some(get_emoji_group_tooltip(group)));
            child = button.next_sibling();
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {}
}

struct EmojiSearchModel {
    all_emojis: gtk4::gio::ListModel,
    filter: gtk4::CustomFilter,
    sorter: gtk4::CustomSorter,
    selection: gtk::SingleSelection,
    grid_view: gtk::GridView,
    matcher: fuzzy_matcher::skim::SkimMatcherV2,
}

#[relm4::component]
impl SimpleComponent for EmojiSearchModel {
    type Init = (gtk::SignalListItemFactory, EmojiArgs);
    type Input = ModeMsg;
    type Output = AppMsg;

    view! {
        gtk::Box {
            add_css_class: "nemu-emojis",
            add_css_class: "nemu-card",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,
            set_vexpand: true,
            set_valign: gtk::Align::Start,

            gtk::ScrolledWindow {
                add_css_class: "nemu-emoji-search",
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_valign: gtk::Align::Start,
                set_vexpand: true,

                #[local_ref]
                matches_grid -> gtk::GridView {
                    set_min_columns: 10,
                    set_max_columns: 30,
                    set_single_click_activate: true,
                    set_valign: gtk::Align::Start,
                    set_tab_behavior: gtk::ListTabBehavior::Item,
                }
            }
        }
    }

    fn init(
        (factory, args): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let all_emojis = crate::emoji::get_all_emojis();
        let filter =
            gtk::CustomFilter::new(|obj| obj.downcast_ref::<EmojiObject>().unwrap().score() > 0);
        let sorter = gtk::CustomSorter::new(|a, b| {
            let a = a.downcast_ref::<EmojiObject>().unwrap().score();
            let b = b.downcast_ref::<EmojiObject>().unwrap().score();
            b.cmp(&a).into()
        });

        let filtered = gtk::FilterListModel::new(Some(all_emojis.clone()), Some(filter.clone()));
        let sorted = gtk::SortListModel::new(Some(filtered), Some(sorter.clone()));
        let selection = gtk::SingleSelection::new(Some(sorted));
        selection.set_autoselect(true);

        let matches_grid = gtk::GridView::default();
        matches_grid.set_factory(Some(&factory));
        matches_grid.set_model(Some(&selection));

        let s = sender.clone();
        matches_grid.connect_activate(move |grid_view, position| {
            let item = grid_view
                .model()
                .unwrap()
                .item(position)
                .and_downcast::<EmojiObject>()
                .unwrap();
            s.output_sender().emit(AppMsg::TextOutput {
                text: item.emoji().as_str().to_owned(),
                notify: args.notify,
            });
        });

        let model = EmojiSearchModel {
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default()
                .ignore_case()
                .use_cache(true),
            filter,
            sorter,
            all_emojis,
            selection,
            grid_view: matches_grid.clone(),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            ModeMsg::SetQueryString(search_string) => {
                let search_string = if search_string.starts_with(':') {
                    &search_string[1..]
                } else {
                    &*search_string
                };

                if search_string.is_empty() {
                    // Reset the score of every emoji.
                    for i in 0..self.all_emojis.n_items() {
                        let emoji = self
                            .all_emojis
                            .item(i)
                            .and_downcast::<EmojiObject>()
                            .unwrap();
                        emoji.set_score(0);
                    }
                } else {
                    for i in 0..self.all_emojis.n_items() {
                        let emoji = self
                            .all_emojis
                            .item(i)
                            .and_downcast::<EmojiObject>()
                            .unwrap();

                        let name_score = self
                            .matcher
                            .fuzzy_match(emoji.emoji().name(), search_string)
                            .unwrap_or(-1);
                        let keyword_score = emoji
                            .emoji()
                            .shortcodes()
                            .filter_map(|shortcode| {
                                self.matcher.fuzzy_match(shortcode, search_string)
                            })
                            .max()
                            .unwrap_or(-1);
                        let score = name_score.max(keyword_score);
                        emoji.set_score(score);
                    }

                    self.filter.changed(gtk::FilterChange::Different);
                    self.sorter.changed(gtk::SorterChange::Different);

                    if let Some(model) = self.grid_view.model()
                        && model.n_items() != 0
                    {
                        self.grid_view.grab_focus();
                        self.selection.set_selected(0);
                        self.grid_view.scroll_to(
                            0,
                            gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                            None,
                        );
                    }
                }
            }
            ModeMsg::ActivateCurrent => {
                let pos = self.selection.selected();
                if pos != gtk::INVALID_LIST_POSITION {
                    self.grid_view.emit_by_name::<()>("activate", &[&pos]);
                }
            }
            _ => (),
        }
    }
}

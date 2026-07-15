use std::{path::Path, rc::Rc, sync::OnceLock};

use freedesktop_desktop_entry::DesktopEntry;
use fuzzy_matcher::FuzzyMatcher as _;
use gtk4::prelude::*;
use relm4::{Sender, prelude::*};

use crate::{
    AppMsg,
    desktop_entry::{DesktopEntryActionKind, DesktopEntryActionObject, DesktopEntryObject},
    get_system_locales,
    mode::{Mode, ModeFactory, ModeMsg},
};

#[derive(Default)]
pub struct AppLauncherModeFactory;

impl ModeFactory for AppLauncherModeFactory {
    fn name(&self) -> &'static str {
        "app_launcher"
    }

    fn create(&self, sender: Sender<AppMsg>, initial_query_string: &str) -> Rc<dyn Mode> {
        let controller = AppLauncherModel::builder()
            .launch(initial_query_string.to_owned())
            .forward(&sender, std::convert::identity);
        Rc::new(AppLauncherMode { controller })
    }

    fn should_switch_to_this_mode(&self, query_string: &str) -> bool {
        query_string.starts_with(char::is_alphabetic)
    }
}

#[derive(Debug)]
pub enum AppsResult {
    LaunchApp(&'static DesktopEntry),
}

struct AppLauncherMode {
    controller: Controller<AppLauncherModel>,
}

impl Mode for AppLauncherMode {
    fn widget(&self) -> &gtk4::Widget {
        self.controller.widget().upcast_ref()
    }

    fn sender(&self) -> &Sender<crate::mode::ModeMsg> {
        self.controller.sender()
    }
}

struct AppLauncherModel {
    apps: gtk::gio::ListModel,
    filter: gtk::CustomFilter,
    sorter: gtk::CustomSorter,
    app_list: gtk::ListView,
    app_selection: gtk::SingleSelection,

    matcher: fuzzy_matcher::skim::SkimMatcherV2,
}

#[relm4::component]
impl SimpleComponent for AppLauncherModel {
    type Init = String;
    type Input = ModeMsg;
    type Output = AppMsg;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 5,

            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_vscrollbar_policy: gtk::PolicyType::Automatic,
                set_propagate_natural_height: true,
                set_max_content_height: 500,
                set_height_request: 500,
                add_css_class: "nemu-card",

                #[local_ref]
                app_list -> gtk::ListView {
                    add_css_class: "nemu-apps",
                    add_css_class: "nemu-results",
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Fill,
                    set_single_click_activate: true,
                    set_tab_behavior: gtk::ListTabBehavior::Item,
                },
            },

            gtk::Revealer {
                set_transition_type: gtk::RevealerTransitionType::SlideRight,
                set_transition_duration: 150,
                #[watch]
                set_reveal_child: model.app_selection.selected_item().is_some(),

                #[name = "details"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: "nemu-card",
                    add_css_class: "nemu-app-details-card",

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Start,

                        #[name = "details_icon_image"]
                        gtk::Image {
                            set_pixel_size: 96,
                            add_css_class: "nemu-app-details-icon",
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_valign: gtk::Align::Center,
                            add_css_class: "nemu-app-details",

                            #[name = "details_app_name"]
                            gtk::Label {
                                add_css_class: "nemu-app-details-name",
                                set_halign: gtk::Align::Start,
                            },
                            #[name = "details_app_description"]
                            gtk::Label {
                                add_css_class: "nemu-app-details-description",
                                set_halign: gtk::Align::Start,
                                set_wrap: true,
                            }
                        }
                    },

                    #[name = "details_actions"]
                    gtk::ListView {
                        set_orientation: gtk::Orientation::Vertical,
                        add_css_class: "nemu-results",
                        add_css_class: "nemu-app-details-actions",
                        set_vexpand: false,
                        set_single_click_activate: true,
                        set_tab_behavior: gtk::ListTabBehavior::Item,
                        set_focusable: false,
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let apps = crate::desktop_entry::get_all_apps();

        let filter = gtk::CustomFilter::new(|obj| {
            obj.downcast_ref::<DesktopEntryObject>().unwrap().score() > 0
        });
        let sorter = gtk::CustomSorter::new(|a, b| {
            let a = a.downcast_ref::<DesktopEntryObject>().unwrap().score();
            let b = b.downcast_ref::<DesktopEntryObject>().unwrap().score();
            b.cmp(&a).into()
        });

        let filtered = gtk::FilterListModel::new(Some(apps.clone()), Some(filter.clone()));
        let sorted = gtk::SortListModel::new(Some(filtered), Some(sorter.clone()));
        let selection = gtk::SingleSelection::new(Some(sorted));
        selection.set_autoselect(true);

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let icon = gtk::Image::builder()
                .pixel_size(32)
                .use_fallback(true)
                .build();
            let app_label = gtk::Label::builder()
                .css_classes(["nemu-app-name"])
                .xalign(0.0)
                .build();
            let app_exec = gtk::Label::builder()
                .css_classes(["nemu-app-command-line"])
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .xalign(0.0)
                .build();

            let vcontainer = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(2)
                .build();
            vcontainer.append(&app_label);
            vcontainer.append(&app_exec);

            let hcontainer = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .css_classes(["nemu-results-row"])
                .spacing(10)
                .build();
            hcontainer.append(&icon);
            hcontainer.append(&vcontainer);
            list_item.set_child(Some(&hcontainer));
        });
        factory.connect_bind(move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let hcontainer = item.child().and_downcast::<gtk::Box>().unwrap();
            let icon = hcontainer
                .first_child()
                .and_downcast::<gtk::Image>()
                .unwrap();
            let vcontainer = icon.next_sibling().unwrap();
            let app_label = vcontainer
                .first_child()
                .and_downcast::<gtk::Label>()
                .unwrap();
            let app_exec = app_label
                .next_sibling()
                .and_downcast::<gtk::Label>()
                .unwrap();

            let desktop_entry = item.item().and_downcast::<DesktopEntryObject>().unwrap();
            let desktop_entry = desktop_entry.entry();
            match desktop_entry.icon() {
                Some(icon_source) => {
                    if let path = Path::new(icon_source)
                        && path.is_absolute()
                    {
                        icon.set_from_file(Some(path));
                    } else {
                        icon.set_icon_name(Some(icon_source));
                    }
                }
                None => icon.set_icon_name(None),
            }

            app_label.set_text(
                desktop_entry
                    .name(crate::get_system_locales())
                    .as_deref()
                    .unwrap_or(&desktop_entry.appid),
            );
            app_exec.set_text(desktop_entry.exec().unwrap_or(""));
        });

        let app_list = gtk::ListView::default();
        app_list.set_model(Some(&selection));
        app_list.set_factory(Some(&factory));
        let s = sender.clone();
        app_list.connect_activate(move |list_view, position| {
            let item = list_view
                .model()
                .unwrap()
                .item(position)
                .and_downcast::<DesktopEntryObject>()
                .unwrap();
            s.output_sender().emit(AppMsg::LaunchDesktopEntry(item));
        });

        let mut model = AppLauncherModel {
            matcher: fuzzy_matcher::skim::SkimMatcherV2::default()
                .ignore_case()
                .score_config(fuzzy_matcher::skim::SkimScoreConfig {
                    penalty_case_mismatch: 0,
                    ..fuzzy_matcher::skim::SkimScoreConfig::default()
                })
                .use_cache(true),
            apps,
            filter,
            sorter,
            app_list: app_list.clone(),
            app_selection: selection,
        };
        let widgets = view_output!();
        model.set_query_string(&init);

        let details_icon_image = widgets.details_icon_image.clone();
        let details_app_name = widgets.details_app_name.clone();
        let details_app_description = widgets.details_app_description.clone();
        let details_actions = widgets.details_actions.clone();
        model
            .app_selection
            .connect_selection_changed(move |app_selection, _, _| {
                let Some(selected) = app_selection
                    .selected_item()
                    .and_downcast::<DesktopEntryObject>()
                else {
                    return;
                };

                let entry = selected.entry();
                match entry.icon() {
                    Some(icon_source) => {
                        if let path = Path::new(icon_source)
                            && path.is_absolute()
                        {
                            details_icon_image.set_from_file(Some(path));
                        } else {
                            details_icon_image.set_icon_name(Some(icon_source));
                        }
                    }
                    None => details_icon_image.set_icon_name(None),
                }

                details_app_name.set_text(
                    entry
                        .name(get_system_locales())
                        .as_deref()
                        .unwrap_or(&entry.appid),
                );
                details_app_description
                    .set_text(entry.comment(get_system_locales()).as_deref().unwrap_or(""));

                let action_selection = gtk::SingleSelection::new(Some(selected.actions()));
                details_actions.set_model(Some(&action_selection));
            });
        let action_factory = gtk::SignalListItemFactory::new();
        action_factory.connect_setup(|_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            list_item.set_child(Some(
                &gtk::Label::builder()
                    .css_classes(["nemu-results-row"])
                    .halign(gtk::Align::Start)
                    .build(),
            ));
        });
        action_factory.connect_bind(|_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
            let item = list_item
                .item()
                .and_downcast::<DesktopEntryActionObject>()
                .unwrap();
            match item.kind() {
                DesktopEntryActionKind::Launch => label.set_text("Launch"),
                DesktopEntryActionKind::DBusActivate => label.set_text("Activate"),
                DesktopEntryActionKind::Named(name, _) => label.set_text(
                    item.entry()
                        .action_name(name, get_system_locales())
                        .as_deref()
                        .unwrap_or(name),
                ),
            }
        });
        widgets.details_actions.set_factory(Some(&action_factory));
        widgets.details_actions.connect_activate(move |list, pos| {
            let Some(selected) = list
                .model()
                .unwrap()
                .item(pos)
                .and_downcast::<DesktopEntryActionObject>()
            else {
                return;
            };
            sender
                .output_sender()
                .emit(AppMsg::LaunchDesktopAction(selected));
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ModeMsg::ActivateCurrent => {
                if let Some(selected) = self
                    .app_selection
                    .selected_item()
                    .and_downcast::<DesktopEntryObject>()
                {
                    _ = sender.output(AppMsg::LaunchDesktopEntry(selected));
                }
            }
            ModeMsg::SetQueryString(gstring) => self.set_query_string(&gstring),
            _ => (),
        }
    }
}

impl AppLauncherModel {
    fn set_query_string(&mut self, query_string: &str) {
        if query_string.is_empty() {
            for i in 0..self.apps.n_items() {
                let desktop_entry = self
                    .apps
                    .item(i)
                    .and_downcast::<DesktopEntryObject>()
                    .unwrap();
                desktop_entry.set_score(0);
            }
        } else {
            let locales = get_system_locales();

            // Calculate a new score for all apps
            for i in 0..self.apps.n_items() {
                let desktop_entry = self
                    .apps
                    .item(i)
                    .and_downcast::<DesktopEntryObject>()
                    .unwrap();
                desktop_entry.set_score(calculate_score(
                    desktop_entry.entry(),
                    query_string,
                    &self.matcher,
                    locales,
                ))
            }
        }

        self.filter.changed(gtk::FilterChange::Different);
        self.sorter.changed(gtk::SorterChange::Different);
        if let Some(model) = self.app_list.model()
            && model.n_items() > 0
        {
            self.app_selection.set_selected(0);
            self.app_list.scroll_to(
                0,
                gtk::ListScrollFlags::SELECT | gtk::ListScrollFlags::FOCUS,
                None,
            );
            self.app_selection.selection_changed(0, 1);
            self.app_list.grab_focus();
        }
    }
}

pub fn cached_desktop_entries() -> &'static [DesktopEntry] {
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

pub fn calculate_score(
    entry: &DesktopEntry,
    pattern: &str,
    matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    locales: &[String],
) -> i64 {
    let Some(name) = entry.name(locales) else {
        return -1;
    };
    let exact_bonus = if name.eq_ignore_ascii_case(pattern) {
        100000
    } else {
        0
    };

    let name_score = matcher.fuzzy_match(&name, pattern).unwrap_or(0) * 10;
    let keyword_score: i64 = entry
        .keywords(locales)
        .iter()
        .flatten()
        .filter_map(|kw| matcher.fuzzy_match(&kw, pattern))
        .max()
        .unwrap_or(0);
    let appid_score = matcher.fuzzy_match(&entry.appid, pattern).unwrap_or(0);

    return exact_bonus
        .max(name_score)
        .max(keyword_score)
        .max(appid_score);
}

use std::{
    collections::{HashMap, hash_map},
    io::Write,
    rc::Rc,
};

use gtk::prelude::*;
use gtk4::{gdk::Key, glib::Propagation};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell as _};
use relm4::prelude::*;

use crate::{
    Args, Command, OutputChoice,
    desktop_entry::{self, DesktopEntryActionObject, DesktopEntryObject},
    get_system_locales,
    mode::{Mode, ModeFactory, ModeMsg},
    modes::{self, EmojiArgs},
    strip_field_codes,
};

pub struct AppModel {
    search_entry: gtk::SearchEntry,
    /// All supported modes for this instance.
    mode_factories: HashMap<&'static str, Rc<dyn ModeFactory>>,
    /// Modes are lazily initialized here.
    initialized_modes: HashMap<&'static str, Rc<dyn Mode>>,
    /// The current mode.
    current_mode: Option<(&'static str, Rc<dyn Mode>)>,
    display: gtk::gdk::Display,
    mode_stack: gtk::Stack,
    command: Option<Command>,
    output: OutputChoice,
}

#[derive(Debug)]
pub enum AppMsg {
    InputChanged,
    ActivateCurrent,
    LaunchDesktopEntry(DesktopEntryObject),
    LaunchDesktopAction(DesktopEntryActionObject),
    // Copy to clipboard or stdout
    TextOutput { text: String, notify: bool },
    Quit,
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = Args;
    type Input = AppMsg;
    type Output = ();

    view! {
        gtk::Window {
            set_title: Some("Nemu"),
            set_default_size: (1, 1),

            gtk::Box {
                add_css_class: "nemu-root",
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                #[local_ref]
                search_entry -> gtk::SearchEntry {
                    connect_search_changed => AppMsg::InputChanged,
                    connect_activate => AppMsg::ActivateCurrent,
                    set_search_delay: 0,
                    set_activates_default: true,
                    add_css_class: "nemu-entry",
                    set_placeholder_text: Some("Type ? for help. Esc to quit."),
                },

                gtk::Revealer {
                    set_transition_type: gtk::RevealerTransitionType::SlideDown,
                    set_transition_duration: 150,
                    #[watch]
                    set_reveal_child: model.current_mode.is_some(),

                    #[local_ref]
                    mode_stack -> gtk::Stack {
                        set_transition_type: gtk::StackTransitionType::Crossfade,
                        set_transition_duration: 150,
                        set_vhomogeneous: false,
                        set_hhomogeneous: false,
                        set_interpolate_size: true,

                        add_child = &gtk::Label { set_label: "" } -> { set_name: "" },

                        #[watch]
                        set_visible_child_name: model.current_mode.as_ref().map(|(name, _)| *name).unwrap_or(""),
                    }
                }
            },
        }
    }

    fn init(
        args: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let has_command = args.command.is_some();

        if !args.windowed {
            // Turn the toplevel into a wlr-layer-shell surface. Must happen before
            // the window is realized/presented (i.e. here in `init`).
            root.init_layer_shell();
            root.set_layer(Layer::Overlay);
            root.set_namespace(Some("nemu"));
            // Grab the keyboard so the launcher can actually receive typed input.
            root.set_keyboard_mode(KeyboardMode::Exclusive);
        }

        let search_entry = gtk::SearchEntry::default();

        // Bind the ESC key to the quit event.
        let key_navigation = gtk::EventControllerKey::new();
        key_navigation.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_navigation.connect_key_pressed(move |_, keyval, _, _| match keyval {
            Key::Escape => {
                relm4::main_application().quit();
                return Propagation::Stop;
            }
            _ => return Propagation::Proceed,
        });
        root.add_controller(key_navigation);

        if !args.windowed {
            let s = sender.clone();
            root.connect_is_active_notify(move |window| {
                if !window.is_active() {
                    s.input(AppMsg::Quit);
                }
            });
        }

        let app_launcher_mode_factory = Rc::new(modes::AppLauncherModeFactory::default());
        let help_factory = Rc::new(modes::HelpModeFactory::default());
        let calc_factory = Rc::new(modes::CalculatorModeFactory::default());
        let emoji_factory = Rc::new(modes::EmojiPickerFactory(
            if let Some(Command::Emoji(emoji_args)) = args.command.as_ref() {
                *emoji_args
            } else {
                EmojiArgs {
                    notify: args.notify,
                }
            },
        ));

        // If passed a mode on the command line, immediately go into that.
        let current_mode: Option<(&'static str, Rc<dyn Mode>)> =
            args.command.as_ref().map(|command| match command {
                Command::Emoji(_) => (
                    emoji_factory.name(),
                    emoji_factory.create(sender.input_sender().clone(), ""),
                ),
                Command::Calc => (
                    calc_factory.name(),
                    calc_factory.create(sender.input_sender().clone(), ""),
                ),
                Command::Dmenu(dmenu_args) => {
                    let strings = dmenu_args.read_strings().unwrap();
                    let dmenu_factory = crate::modes::DmenuModeFactory(dmenu_args.clone(), strings);
                    (
                        dmenu_factory.name(),
                        dmenu_factory.create(sender.input_sender().clone(), ""),
                    )
                }
            });

        let mode_factories: HashMap<&'static str, Rc<dyn ModeFactory>> = if !has_command {
            [
                app_launcher_mode_factory as Rc<dyn ModeFactory>,
                help_factory as _,
                calc_factory as _,
                emoji_factory as _,
            ]
            .into_iter()
            .map(|f| (f.name(), f as _))
            .collect()
        } else {
            Default::default()
        };

        let mode_stack = gtk::Stack::default();
        let mut model = AppModel {
            search_entry: search_entry.clone(),
            mode_factories,
            current_mode: None,
            initialized_modes: HashMap::default(),
            display: RootExt::display(&root),
            mode_stack: mode_stack.clone(),
            command: args.command,
            output: args.output,
        };
        let widgets = view_output!();

        // Activate the initial mode if one exists.
        if let Some((mode_name, current_mode)) = current_mode {
            widgets
                .search_entry
                .set_placeholder_text(Some(&"Type to search"));
            widgets
                .mode_stack
                .add_named(current_mode.widget(), Some(mode_name));
            _ = current_mode.sender().send(ModeMsg::Activate);
            sender.input_sender().emit(AppMsg::InputChanged);
            model.current_mode = Some((mode_name, current_mode));
        }

        model.search_entry.set_key_capture_widget(Some(&root));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            AppMsg::Quit => relm4::main_application().quit(),
            AppMsg::ActivateCurrent => {
                tracing::debug!("Activate current");
                if let Some((_, current_mode)) = self.current_mode.as_ref() {
                    current_mode.sender().emit(ModeMsg::ActivateCurrent);
                }
            }
            AppMsg::InputChanged => {
                let text = self.search_entry.text();
                tracing::debug!("Search string changed: {text}");

                if text.is_empty() && self.command.is_none() {
                    if let Some((_, current_mode)) = self.current_mode.as_ref() {
                        _ = current_mode.sender().send(ModeMsg::Deactivate);
                        self.current_mode = None;
                        tracing::debug!("Switched to mode: empty");
                    }
                    return;
                }

                // Find a matching mode factory
                if self.current_mode.is_none() {
                    for (mode_name, factory) in self.mode_factories.iter() {
                        if factory.should_switch_to_this_mode(&text) {
                            let new_current = match self.initialized_modes.entry(mode_name) {
                                hash_map::Entry::Occupied(entry) => entry.get().clone(),
                                hash_map::Entry::Vacant(entry) => {
                                    let new_current =
                                        factory.create(sender.input_sender().clone(), &text);
                                    entry.insert(new_current.clone());
                                    self.mode_stack
                                        .add_named(new_current.widget(), Some(*mode_name));
                                    new_current
                                }
                            };

                            _ = new_current.sender().send(ModeMsg::Activate);
                            self.current_mode = Some((mode_name, new_current));
                            tracing::debug!("Switched to mode: {mode_name}");
                            break;
                        }
                    }
                }

                if let Some((_, current_mode)) = self.current_mode.as_ref() {
                    _ = current_mode
                        .sender()
                        .send(ModeMsg::SetQueryString(text.to_owned()));
                }
            }
            AppMsg::LaunchDesktopEntry(desktop_entry) => {
                self.launch_desktop_entry(desktop_entry);
                relm4::main_application().quit();
            }
            AppMsg::LaunchDesktopAction(desktop_action) => {
                self.launch_desktop_entry_action(desktop_action);
                relm4::main_application().quit();
            }
            AppMsg::TextOutput { text, notify } => {
                self.text_output(&text, notify);
                relm4::main_application().quit();
            }
        }
    }
}

impl AppModel {
    fn text_output(&self, text: &str, notify: bool) {
        match self.output {
            OutputChoice::Clipboard => {
                // Spawn a subprocess instead of using the Gtk clipboard
                // interface directly, because we are likely immediately exiting.
                let mut child = std::process::Command::new("wl-copy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .expect("wl-copy");
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(text.as_bytes())
                    .unwrap();

                if notify {
                    let notification = gtk::gio::Notification::new(&format!("Nemu"));
                    notification.set_priority(gtk::gio::NotificationPriority::Low);
                    notification.set_body(Some(&format!("Copied to clipboard: {text}")));
                    relm4::main_application()
                        .send_notification(Some("dev.nemu.Nemu"), &notification);
                }
            }
            OutputChoice::Stdout => _ = std::io::stdout().write_all(text.as_bytes()),
            OutputChoice::Stderr => _ = std::io::stderr().write_all(text.as_bytes()),
        }
    }

    fn launch_desktop_entry(&self, desktop_entry: DesktopEntryObject) {
        self.launch_desktop_entry_(desktop_entry.entry());
    }

    fn launch_desktop_entry_(&self, desktop_entry: &freedesktop_desktop_entry::DesktopEntry) {
        tracing::debug!("Launching desktop entry: {}", desktop_entry.id());
        let Some(exec) = desktop_entry.exec() else {
            tracing::error!(
                "Cannot launch {}: No Exec section in desktop entry",
                desktop_entry.appid
            );
            return;
        };
        let command_line = strip_field_codes(exec);
        let app_info = gtk::gio::AppInfo::create_from_commandline(
            command_line,
            desktop_entry.name(get_system_locales()).as_deref(),
            gtk::gio::AppInfoCreateFlags::NONE,
        )
        .unwrap();
        let ctx = self.display.app_launch_context();
        app_info.launch(&[], Some(&ctx)).unwrap();
    }

    fn launch_desktop_entry_action(&self, action: DesktopEntryActionObject) {
        match action.kind() {
            desktop_entry::DesktopEntryActionKind::Launch => {
                self.launch_desktop_entry_(action.entry())
            }
            desktop_entry::DesktopEntryActionKind::DBusActivate => todo!(),
            desktop_entry::DesktopEntryActionKind::Named(name, exec) => {
                tracing::debug!("Running action: {}:{name}", action.entry().appid);
                let command_line = strip_field_codes(exec);
                let app_info = gtk::gio::AppInfo::create_from_commandline(
                    command_line,
                    action.entry().name(get_system_locales()).as_deref(),
                    gtk::gio::AppInfoCreateFlags::NONE,
                )
                .unwrap();
                let ctx = self.display.app_launch_context();
                app_info.launch(&[], Some(&ctx)).unwrap();
            }
        }
    }
}

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use clap::Parser;
use relm4::prelude::*;

use crate::modes::{DmenuArgs, EmojiArgs};

mod app;
mod config;
mod desktop_entry;
mod emoji;
mod mode;
pub mod modes;

pub use app::*;

pub fn get_system_locales() -> &'static [String] {
    static SYSTEM_LOCALES: OnceLock<Vec<String>> = OnceLock::new();
    SYSTEM_LOCALES.get_or_init(freedesktop_desktop_entry::get_languages_from_env)
}

/// Optional subcommand.
#[derive(clap::Subcommand, Clone)]
enum Command {
    /// Open the Emoji picker. Writes the selected emoji to stdout.
    Emoji(EmojiArgs),
    /// Open the calculator
    Calc,
    /// Read lines from stdin, write the selected line to stdout.
    Dmenu(DmenuArgs),
}

/// Nemu app launcher and swiss army knife
#[derive(clap::Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Choose where to put text output, such as that from the Emoji picker.
    #[clap(long, short, default_value = "clipboard")]
    output: OutputChoice,

    /// Show a notification when something was copied to the clipboard.
    #[clap(long, default_value = "false")]
    notify: bool,

    /// Use config file instead of $HOME/.config/nemu/config.toml.
    #[clap(long)]
    config: Option<PathBuf>,

    /// Use CSS from this file instead of $HOME/.config/nemu/style.css.
    #[clap(long)]
    style: Option<PathBuf>,

    /// Show Nemu as a regular window instead of a shell layer on top of other windows.
    /// This is mainly useful for debugging.
    #[clap(long)]
    windowed: bool,
}

#[derive(Default, Clone, Copy, clap::ValueEnum)]
pub enum OutputChoice {
    #[default]
    Clipboard,
    Stdout,
    Stderr,
}

fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| Path::new(&home).join(".config")))
        .ok();
    if config_home.is_none() {
        tracing::warn!("Neither $XDG_CONFIG_HOME nor $HOME are set.")
    }
    let config_path = match args.config.as_ref() {
        Some(config_path) => {
            if !config_path.is_file() {
                tracing::error!("Config path not found: {}", config_path.display());
                None
            } else {
                Some(config_path.clone())
            }
        }
        None => config_home
            .as_ref()
            .map(|ch| ch.join("nemu").join("config.toml"))
            .filter(|p| p.exists()),
    };
    let style_path = match args.style.as_ref() {
        Some(style_path) => {
            if !style_path.is_file() {
                tracing::error!("Style path not found: {}", style_path.display());
                None
            } else {
                Some(style_path.clone())
            }
        }
        None => config_home
            .as_ref()
            .map(|ch| ch.join("nemu").join("style.css"))
            .filter(|p| p.exists()),
    };

    let config = if let Some(config_path) = config_path {
        tracing::debug!("Using configuration file: {}", config_path.display());
        let toml = std::fs::read_to_string(config_path).unwrap();
        toml::from_str(&toml).unwrap()
    } else {
        tracing::debug!("No user config; using default");
        config::Config::default()
    };

    let app = RelmApp::new("org.nemu.Nemu")
        // Avoid passing environment argv, because GLib tries to handle them instead of clap.
        .with_args(vec![std::env::args().next().unwrap_or_default()]);

    if let Some(gtk_settings) = gtk::Settings::default() {
        gtk_settings.set_gtk_application_prefer_dark_theme(!config.light);
    }

    // Safe here: RelmApp::new() has already called gtk::init(), so a default
    // display exists. include_str! compiles the stylesheet into the binary.
    relm4::set_global_css(include_str!("style.css"));
    if let Some(style_path) = style_path {
        tracing::debug!("Using stylesheet: {}", style_path.display());
        _ = relm4::set_global_css_from_file(style_path);
    } else {
        tracing::debug!("No user stylesheet; using default style");
    }

    app.run::<AppModel>(args);
}

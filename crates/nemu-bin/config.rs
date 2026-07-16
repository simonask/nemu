#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
   /// Set the GTK base theme to light. Default is false.
    #[serde(default)]
    pub light: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { light: false }
    }
}

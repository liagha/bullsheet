use std::sync::{Arc, Mutex};

use element::Text;

const OWNER: &str = "liagha";
const REPOSITORY: &str = "bullsheet";
const PROGRAM: &str = "bullsheet";
const BUNDLE: &str = "bullsheet.app";
const TARGET: &str = if cfg!(target_os = "windows") {
    "windows-x86_64"
} else if cfg!(target_os = "macos") {
    "macos-universal"
} else {
    "linux-x86_64"
};

#[derive(Clone)]
pub struct Update {
    state: Arc<Mutex<Option<Text>>>,
}

impl Update {
    pub fn new(wake: impl Fn() + Send + 'static) -> Self {
        let state = Arc::new(Mutex::new(None));
        let worker = Arc::clone(&state);
        std::thread::spawn(move || {
            let message = Self::install_newer();
            if let Ok(mut slot) = worker.lock() {
                *slot = Some(message);
            }
            wake();
        });
        Self { state }
    }

    pub fn text(&self) -> Text {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.clone())
            .unwrap_or_else(|| Text::new("checking"))
    }

    fn install_newer() -> Text {
        use self_update::VersionStatus;
        use self_update::backends::github::Update as Release;
        let current = env!("CARGO_PKG_VERSION");
        let mut builder = Release::configure();
        builder
            .repo_owner(OWNER)
            .repo_name(REPOSITORY)
            .bin_name(PROGRAM)
            .current_version(current)
            .target(TARGET)
            .show_download_progress(false)
            .show_output(false)
            .no_confirm(true);
        if cfg!(target_os = "macos") {
            builder.bundle_path_in_archive(BUNDLE);
        }
        let release = match builder.build() {
            Ok(release) => release,
            Err(error) => return Text::new(format!("update check failed: {error}")),
        };
        match release.update() {
            Ok(VersionStatus::Updated(version)) => {
                Text::new(format!("updated to {version} — restart to apply"))
            }
            Ok(_) => Text::new("up to date"),
            Err(error) => Text::new(format!("update check failed: {error}")),
        }
    }
}

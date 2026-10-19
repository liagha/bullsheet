use crate::render::Presentation;
use element::Failure;

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

#[derive(Debug)]
pub(crate) struct Version(Presentation);

impl Version {
    pub(crate) fn current() -> Self {
        Self(Presentation::new(env!("CARGO_PKG_VERSION")))
    }

    fn new(version: impl Into<String>) -> Self {
        Self(Presentation::new(version))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug)]
pub(crate) enum Report {
    Current,
    Updated(Version),
    Failed(Failure),
}

pub(crate) struct Updater {
    reports: std::sync::mpsc::Receiver<Report>,
}

impl Updater {
    pub(crate) fn new(context: &eframe::egui::Context) -> Self {
        let (sender, reports) = std::sync::mpsc::channel();
        let wake = context.clone();
        std::thread::spawn(move || {
            let _ = sender.send(Self::install());
            wake.request_repaint();
        });
        Self { reports }
    }

    pub(crate) fn report(&self) -> Option<Report> {
        self.reports.try_recv().ok()
    }

    fn install() -> Report {
        use self_update::VersionStatus;
        use self_update::backends::github::Update;
        let current = Version::current();
        let mut builder = Update::configure();
        builder
            .repo_owner(OWNER)
            .repo_name(REPOSITORY)
            .bin_name(PROGRAM)
            .current_version(current.to_string())
            .target(TARGET)
            .show_download_progress(false)
            .show_output(false)
            .no_confirm(true);
        if cfg!(target_os = "macos") {
            builder.bundle_path_in_archive(BUNDLE);
        }
        let update = match builder.build() {
            Ok(update) => update,
            Err(error) => return Report::Failed(Failure::new(error.to_string())),
        };
        match update.update() {
            Ok(VersionStatus::Updated(version)) => {
                Report::Updated(Version::new(version))
            }
            Ok(_) => Report::Current,
            Err(error) => Report::Failed(Failure::new(error.to_string())),
        }
    }
}

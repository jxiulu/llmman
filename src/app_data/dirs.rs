use std::{
    env, fs,
    path::{
        Path, PathBuf
    },
    sync::OnceLock
};

pub struct AppDirectories {
    config: PathBuf,
    saves: PathBuf,
}
impl AppDirectories {
    pub fn config(&self) -> &Path {
        &self.config
    }
    pub fn saves(&self) -> &Path {
        &self.saves
    }
}

static APP_DIRECTORIES: OnceLock<AppDirectories> = OnceLock::new();

pub fn dirs() -> &'static AppDirectories {
    APP_DIRECTORIES.get_or_init(|| {
        let exe = env::current_exe().unwrap();
        let parent = exe.parent().unwrap();

        let config_dir = parent.join("config");
        let saves_dir = parent.join("saves");

        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(&saves_dir).unwrap();

        AppDirectories { config: config_dir, saves: saves_dir }
    })
}

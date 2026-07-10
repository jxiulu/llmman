use std::{
    env, fs, process::Command
};

use color_eyre as eyre;

const PATH: &str = "tmp/setboy_edit.md";

/// suspend terminal before, resume after
pub fn start(content: &str) -> eyre::Result<String> {
    let current_exe = env::current_exe()?;
    let tmp_file_path = current_exe.join(PATH);
    fs::write(&tmp_file_path, content)?;

    let editor = env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());
    Command::new(&editor)
        .arg(PATH)
        .status()?;

    Ok(fs::read_to_string(tmp_file_path)?)
}

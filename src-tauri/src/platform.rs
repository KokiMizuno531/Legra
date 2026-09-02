use std::{env, path::PathBuf};

pub const PLATFORM_NAME: &str = if cfg!(target_os = "macos") {
    "macos"
} else if cfg!(target_os = "windows") {
    "windows"
} else if cfg!(target_os = "linux") {
    "linux"
} else {
    "unknown"
};

pub const PATH_SEPARATOR: &str = if cfg!(target_os = "windows") {
    "\\"
} else {
    "/"
};

pub const NATIVE_HOST_BINARY_NAME: &str = if cfg!(target_os = "windows") {
    "paper_manager_native_host.exe"
} else {
    "paper_manager_native_host"
};

pub const MCP_SERVER_BINARY_NAME: &str = if cfg!(target_os = "windows") {
    "legra_mcp.exe"
} else {
    "legra_mcp"
};

fn nonempty_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub fn home_dir() -> Result<PathBuf, String> {
    nonempty_env_path("HOME")
        .or_else(|| nonempty_env_path("USERPROFILE"))
        .ok_or_else(|| "Could not resolve the user home directory.".to_string())
}

pub fn default_setting_dir() -> Result<PathBuf, String> {
    if let Some(path) = nonempty_env_path("LEGRA_SETTING_DIR") {
        return Ok(path);
    }

    #[cfg(target_os = "macos")]
    {
        return Ok(home_dir()?
            .join("Library")
            .join("Application Support")
            .join("Legra"));
    }

    #[cfg(target_os = "windows")]
    {
        let roaming =
            nonempty_env_path("APPDATA").unwrap_or(home_dir()?.join("AppData").join("Roaming"));
        return Ok(roaming.join("Legra"));
    }

    #[cfg(target_os = "linux")]
    {
        let data_home =
            nonempty_env_path("XDG_DATA_HOME").unwrap_or(home_dir()?.join(".local").join("share"));
        return Ok(data_home.join("Legra"));
    }

    #[allow(unreachable_code)]
    Ok(home_dir()?.join(".legra"))
}

fn select_setting_dir(
    explicit_override: Option<PathBuf>,
    default_dir: PathBuf,
    legacy_dir: PathBuf,
) -> PathBuf {
    if let Some(path) = explicit_override {
        return path;
    }

    let default_data = default_dir.join("app-data.json");
    let legacy_data = legacy_dir.join("app-data.json");
    if legacy_data.exists() && !default_data.exists() {
        legacy_dir
    } else {
        default_dir
    }
}

pub fn setting_dir_with_legacy(legacy_dir: PathBuf) -> Result<PathBuf, String> {
    let explicit_override = nonempty_env_path("LEGRA_SETTING_DIR");
    let default_dir = match explicit_override.as_ref() {
        Some(path) => path.clone(),
        None => default_setting_dir()?,
    };
    Ok(select_setting_dir(
        explicit_override,
        default_dir,
        legacy_dir,
    ))
}

pub fn user_config_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        return Ok(home_dir()?.join("Library").join("Application Support"));
    }

    #[cfg(target_os = "windows")]
    {
        return nonempty_env_path("APPDATA")
            .or_else(|| {
                home_dir()
                    .ok()
                    .map(|home| home.join("AppData").join("Roaming"))
            })
            .ok_or_else(|| "Could not resolve the user configuration directory.".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        return Ok(nonempty_env_path("XDG_CONFIG_HOME").unwrap_or(home_dir()?.join(".config")));
    }

    #[allow(unreachable_code)]
    home_dir()
}

#[cfg(test)]
mod tests {
    use super::select_setting_dir;
    use std::path::PathBuf;

    #[test]
    fn explicit_setting_dir_wins_before_legacy_fallback() {
        let explicit = PathBuf::from("explicit-setting");
        let selected = select_setting_dir(
            Some(explicit.clone()),
            PathBuf::from("default-setting"),
            PathBuf::from("legacy-setting"),
        );

        assert_eq!(selected, explicit);
    }
}

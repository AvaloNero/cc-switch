use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const LANGUAGE_MODELS_FILE: &str = "chatLanguageModels.json";
const MAX_DISCOVERED_TARGETS: usize = 64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum VsCodeEdition {
    Stable,
    Insiders,
}

impl VsCodeEdition {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Insiders => "insiders",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Stable => "Visual Studio Code",
            Self::Insiders => "Visual Studio Code Insiders",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VsCodeProfileTarget {
    pub id: String,
    pub edition: VsCodeEdition,
    pub edition_name: String,
    pub profile_id: Option<String>,
    pub profile_name: String,
    pub is_default: bool,
    pub user_dir: String,
    pub language_models_path: String,
    pub config_exists: bool,
    pub backup_exists: bool,
}

impl VsCodeProfileTarget {
    pub fn path(&self) -> PathBuf {
        PathBuf::from(&self.language_models_path)
    }
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.cc-switch.bak")
}

fn target(
    edition: VsCodeEdition,
    user_dir: &Path,
    profile_id: Option<String>,
    profile_dir: &Path,
) -> VsCodeProfileTarget {
    let is_default = profile_id.is_none();
    let profile_name = profile_id
        .as_deref()
        .map(|id| format!("Profile {id}"))
        .unwrap_or_else(|| "Default".to_string());
    let id = match profile_id.as_deref() {
        Some(profile_id) => format!("{}:profile:{profile_id}", edition.slug()),
        None => format!("{}:default", edition.slug()),
    };
    let language_models_path = profile_dir.join(LANGUAGE_MODELS_FILE);

    VsCodeProfileTarget {
        id,
        edition,
        edition_name: edition.display_name().to_string(),
        profile_id,
        profile_name,
        is_default,
        user_dir: user_dir.to_string_lossy().to_string(),
        config_exists: language_models_path.exists(),
        backup_exists: backup_path(&language_models_path).exists(),
        language_models_path: language_models_path.to_string_lossy().to_string(),
    }
}

fn default_user_roots() -> Vec<(VsCodeEdition, PathBuf)> {
    let Some(config_dir) = dirs::config_dir() else {
        return Vec::new();
    };

    vec![
        (VsCodeEdition::Stable, config_dir.join("Code").join("User")),
        (
            VsCodeEdition::Insiders,
            config_dir.join("Code - Insiders").join("User"),
        ),
    ]
}

pub fn discover_vscode_targets() -> Result<Vec<VsCodeProfileTarget>, AppError> {
    discover_from_roots(&default_user_roots())
}

pub(crate) fn discover_from_roots(
    roots: &[(VsCodeEdition, PathBuf)],
) -> Result<Vec<VsCodeProfileTarget>, AppError> {
    let mut targets = Vec::new();

    for (edition, user_dir) in roots {
        if !user_dir.is_dir() {
            continue;
        }

        targets.push(target(*edition, user_dir, None, user_dir));
        if targets.len() >= MAX_DISCOVERED_TARGETS {
            break;
        }

        let profiles_dir = user_dir.join("profiles");
        if !profiles_dir.is_dir() {
            continue;
        }

        let mut entries = fs::read_dir(&profiles_dir)
            .map_err(|error| AppError::io(&profiles_dir, error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::io(&profiles_dir, error))?;
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            if targets.len() >= MAX_DISCOVERED_TARGETS {
                break;
            }

            let file_type = entry
                .file_type()
                .map_err(|error| AppError::io(entry.path(), error))?;
            if !file_type.is_dir() || file_type.is_symlink() {
                continue;
            }

            let profile_id = entry.file_name().to_string_lossy().trim().to_string();
            if profile_id.is_empty() {
                continue;
            }

            targets.push(target(*edition, user_dir, Some(profile_id), &entry.path()));
        }
    }

    targets.sort_by(|left, right| {
        left.edition
            .cmp(&right.edition)
            .then_with(|| right.is_default.cmp(&left.is_default))
            .then_with(|| left.profile_name.cmp(&right.profile_name))
    });
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_default_and_named_profiles() {
        let temp = tempfile::tempdir().expect("temp directory");
        let user_dir = temp.path().join("Code").join("User");
        let profile_dir = user_dir.join("profiles").join("work-profile");
        fs::create_dir_all(&profile_dir).expect("create profile directory");
        fs::write(user_dir.join(LANGUAGE_MODELS_FILE), "[]").expect("write default config");

        let targets = discover_from_roots(&[(VsCodeEdition::Stable, user_dir.clone())])
            .expect("discover targets");

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].id, "stable:default");
        assert!(targets[0].config_exists);
        assert_eq!(targets[1].id, "stable:profile:work-profile");
        assert_eq!(targets[1].path(), profile_dir.join(LANGUAGE_MODELS_FILE));
    }

    #[test]
    fn ignores_missing_installations() {
        let temp = tempfile::tempdir().expect("temp directory");
        let targets = discover_from_roots(&[(
            VsCodeEdition::Insiders,
            temp.path().join("missing").join("User"),
        )])
        .expect("discover targets");
        assert!(targets.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinked_profile_directories() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("temp directory");
        let user_dir = temp.path().join("Code").join("User");
        let profiles_dir = user_dir.join("profiles");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&profiles_dir).expect("create profiles directory");
        fs::create_dir_all(&outside).expect("create outside directory");
        symlink(&outside, profiles_dir.join("linked")).expect("create symlink");

        let targets =
            discover_from_roots(&[(VsCodeEdition::Stable, user_dir)]).expect("discover targets");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "stable:default");
    }
}

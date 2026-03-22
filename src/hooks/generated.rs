use std::fs;
use std::path::{Path, PathBuf};

use crate::hooks::InstallHooksError;

const SUPPORTED_GENERATED_HOOKS: &[&str] = &["pre-push", "update"];

pub(super) fn validate_supported_hooks(
    paths: &crate::config::LoadPaths,
    hooks: &[String],
) -> Result<(), InstallHooksError> {
    for hook in hooks {
        if !SUPPORTED_GENERATED_HOOKS.contains(&hook.as_str()) {
            return Err(InstallHooksError::UnsupportedGeneratedHook {
                paths: paths.clone(),
                hook: hook.clone(),
            });
        }
    }

    Ok(())
}

pub(super) fn generated_hooks_dir(common_dir: &Path) -> PathBuf {
    common_dir.join("repocert").join("hooks").join("generated")
}

pub(super) fn sync_generated_hooks(
    paths: &crate::config::LoadPaths,
    hooks_dir: &Path,
    hooks: &[String],
    executable_path: &Path,
) -> Result<Vec<String>, InstallHooksError> {
    fs::create_dir_all(hooks_dir).map_err(|source| InstallHooksError::GeneratedHookWrite {
        paths: paths.clone(),
        hook: "directory".to_string(),
        path: hooks_dir.to_path_buf(),
        source,
    })?;

    let mut repaired = Vec::new();
    for hook in hooks {
        let path = hooks_dir.join(hook);
        let content = hook_script(hook, executable_path, &paths.repo_root);
        let needs_write = match fs::read_to_string(&path) {
            Ok(existing) => existing != content,
            Err(_) => true,
        };

        if needs_write {
            fs::write(&path, content).map_err(|source| InstallHooksError::GeneratedHookWrite {
                paths: paths.clone(),
                hook: hook.clone(),
                path: path.clone(),
                source,
            })?;
            set_executable(&path).map_err(|source| InstallHooksError::GeneratedHookWrite {
                paths: paths.clone(),
                hook: hook.clone(),
                path: path.clone(),
                source,
            })?;
            repaired.push(format!("generated hook {hook}"));
        }
    }

    let desired = hooks
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for entry in
        fs::read_dir(hooks_dir).map_err(|source| InstallHooksError::GeneratedHookPrune {
            paths: paths.clone(),
            path: hooks_dir.to_path_buf(),
            source,
        })?
    {
        let entry = entry.map_err(|source| InstallHooksError::GeneratedHookPrune {
            paths: paths.clone(),
            path: hooks_dir.to_path_buf(),
            source,
        })?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !desired.contains(&file_name) && entry.path().is_file() {
            fs::remove_file(entry.path()).map_err(|source| {
                InstallHooksError::GeneratedHookPrune {
                    paths: paths.clone(),
                    path: entry.path(),
                    source,
                }
            })?;
            repaired.push(format!("removed stale generated hook {file_name}"));
        }
    }

    Ok(repaired)
}

fn hook_script(hook: &str, executable_path: &Path, repo_root: &Path) -> String {
    let exe = shell_quote(executable_path);
    let repo = shell_quote(repo_root);
    match hook {
        "update" => format!(
            "#!/bin/sh\nset -eu\nexec {exe} authorize \"$2\" \"$3\" \"$1\" --repo-root {repo}\n"
        ),
        "pre-push" => format!(
            "#!/bin/sh\nset -eu\nremote_name=\"$1\"\nremote_location=\"$2\"\nwhile IFS=' ' read -r local_ref local_oid remote_ref remote_oid; do\n    [ -z \"$local_ref\" ] && continue\n    {exe} authorize \"$remote_oid\" \"$local_oid\" \"$remote_ref\" --repo-root {repo} || exit $?\ndone\n"
        ),
        other => unreachable!("unsupported generated hook {other}"),
    }
}

fn shell_quote(path: &Path) -> String {
    let raw = path.display().to_string();
    format!("'{}'", raw.replace('\'', "'\"'\"'"))
}

#[cfg(unix)]
fn set_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

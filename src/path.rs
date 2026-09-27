use std::path::{Component, Path, PathBuf, is_separator};

const HOME_PREFIX: char = '~';

/// The field before and including its last separator, and what follows.
pub(crate) fn split_at_separator(text: &str) -> (&str, &str) {
    text.rfind(is_separator)
        .map_or(("", text), |at| (&text[..=at], &text[at + 1..]))
}

/// The directory the field names, with `~` expanded, resolved against
/// `base`. Empty when it is a relative base itself; see [`resolve`].
pub(crate) fn field_directory(text: &str, base: &Path) -> PathBuf {
    resolve(&expand_home(split_at_separator(text).0), base)
}

/// `path` joined to `base` when relative, lexically normalized. The result
/// is empty for the working directory, which keeps prefix checks honest;
/// [`or_current`] names it for the filesystem.
pub(crate) fn resolve(path: &Path, base: &Path) -> PathBuf {
    normalize(&base.join(path))
}

/// Whether `directory` has a parent to climb to, judged on its absolute
/// form so a `..` chain below a relative base stops at the root. A relative
/// directory with no working directory to resolve it against has none.
pub(crate) fn has_parent(directory: &Path, cwd: Option<PathBuf>) -> bool {
    if directory.is_absolute() {
        return directory.parent().is_some();
    }
    cwd.is_some_and(|cwd| resolve(directory, &cwd).parent().is_some())
}

/// `.` for the empty path [`resolve`] hands back, `path` otherwise.
pub(crate) fn or_current(path: PathBuf) -> PathBuf {
    if path.as_os_str().is_empty() {
        PathBuf::from(Component::CurDir.as_os_str())
    } else {
        path
    }
}

fn expand_home(text: &str) -> PathBuf {
    let Some(rest) = text.strip_prefix(HOME_PREFIX) else {
        return PathBuf::from(text);
    };
    let is_home = rest.is_empty() || rest.starts_with(is_separator);
    match (is_home, std::env::home_dir()) {
        (true, Some(home)) => home.join(rest.trim_start_matches(is_separator)),
        _ => PathBuf::from(text),
    }
}

/// Lexical normalization: `.` dropped, `..` pops the segment before it,
/// stays at a root, and accumulates below a relative start. A symlink can
/// still lead outside what this reports; the floor is a lexical one.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match normalized.components().next_back() {
                None | Some(Component::ParentDir) => normalized.push(component),
                Some(Component::RootDir | Component::Prefix(_)) => {}
                Some(_) => {
                    normalized.pop();
                }
            },
            other => normalized.push(other),
        }
    }
    normalized
}

/// The field text naming `directory`, with its trailing separator:
/// relative to `base` when beneath it, climbing with `..` when both are
/// relative, absolute otherwise.
pub(crate) fn directory_text(directory: &Path, base: &Path) -> String {
    let base = normalize(base);
    let mut climb = PathBuf::new();
    let mut ancestor = base.as_path();
    loop {
        if let Ok(below) = directory.strip_prefix(ancestor) {
            return climb.join(below).join("").display().to_string();
        }
        match ancestor.parent() {
            Some(parent) if !directory.is_absolute() => {
                ancestor = parent;
                climb.push(Component::ParentDir);
            }
            _ => return directory.join("").display().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        directory_text, field_directory, has_parent, normalize, or_current, split_at_separator,
    };
    use std::path::{Path, PathBuf};

    fn directory(text: &str) -> PathBuf {
        field_directory(text, Path::new("/base"))
    }

    #[test]
    fn splits_after_the_last_separator() {
        assert_eq!(split_at_separator(""), ("", ""));
        assert_eq!(split_at_separator("ma"), ("", "ma"));
        assert_eq!(split_at_separator("src/ma"), ("src/", "ma"));
        assert_eq!(split_at_separator("/"), ("/", ""));
    }

    #[test]
    fn empty_field_names_base() {
        assert_eq!(directory(""), PathBuf::from("/base"));
        assert_eq!(directory("ma"), PathBuf::from("/base"));
    }

    #[test]
    fn relative_directory_joins_base_and_absolute_replaces_it() {
        assert_eq!(directory("src/ma"), PathBuf::from("/base/src"));
        assert_eq!(directory("/etc/"), PathBuf::from("/etc"));
        assert_eq!(directory("/"), PathBuf::from("/"));
    }

    #[test]
    fn parent_segments_traverse_above_base() {
        assert_eq!(directory("../"), PathBuf::from("/"));
        assert_eq!(directory("../../x/"), PathBuf::from("/x"));
    }

    #[test]
    fn tilde_expands_to_home() {
        let home = std::env::home_dir().expect("a home directory for tests");
        assert_eq!(directory("~/"), home);
        assert_eq!(directory("~/d/f"), home.join("d"));
        assert_eq!(directory("~x/"), PathBuf::from("/base/~x"));
    }

    #[test]
    fn a_relative_base_resolves_to_the_empty_path_named_as_dot() {
        assert_eq!(field_directory("", Path::new(".")), PathBuf::new());
        assert_eq!(field_directory("../", Path::new("x")), PathBuf::new());
        assert_eq!(or_current(PathBuf::new()), PathBuf::from("."));
        assert_eq!(or_current(PathBuf::from("x")), PathBuf::from("x"));
    }

    #[test]
    fn directory_text_is_relative_beneath_the_base_and_climbs_above_it() {
        let base = Path::new("/base");
        assert_eq!(directory_text(Path::new("/base"), base), "");
        assert_eq!(directory_text(Path::new("/base/src"), base), "src/");
        assert_eq!(directory_text(Path::new("/"), base), "/");
        assert_eq!(directory_text(Path::new("/etc"), base), "/etc/");
        assert_eq!(directory_text(Path::new(".."), Path::new(".")), "../");
        assert_eq!(directory_text(Path::new("x"), Path::new("x/y")), "../");
        assert_eq!(directory_text(Path::new("z"), Path::new("x")), "../z/");
        assert_eq!(
            directory_text(Path::new("../b"), Path::new("../a")),
            "../b/"
        );
    }

    #[test]
    fn has_parent_is_judged_on_the_absolute_form() {
        let cwd = || std::env::current_dir().ok();
        assert!(!has_parent(Path::new("/"), cwd()));
        assert!(has_parent(Path::new("/x"), cwd()));
        assert!(has_parent(Path::new(""), cwd()));
    }

    #[test]
    fn has_parent_needs_no_working_directory_for_an_absolute_path() {
        assert!(has_parent(Path::new("/x"), None));
        assert!(!has_parent(Path::new("/"), None));
        assert!(!has_parent(Path::new("x"), None));
    }

    #[test]
    fn normalize_folds_dots_and_keeps_leading_parents() {
        assert_eq!(normalize(Path::new("a/./b/../c")), PathBuf::from("a/c"));
        assert_eq!(normalize(Path::new("../../a")), PathBuf::from("../../a"));
        assert_eq!(normalize(Path::new("a/../..")), PathBuf::from(".."));
        assert_eq!(normalize(Path::new("/..")), PathBuf::from("/"));
    }
}

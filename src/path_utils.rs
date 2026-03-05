use std::path::{Path, PathBuf};

pub fn extract_command_binary(command: &str) -> Option<String> {
    let tokens = shell_like_split(command);
    for token in tokens {
        if is_env_assignment(&token) {
            continue;
        }
        if !token.is_empty() {
            return Some(token);
        }
    }
    None
}

pub fn resolve_binary_in_path(binary: &str) -> Option<PathBuf> {
    if binary.trim().is_empty() {
        return None;
    }

    let binary_path = Path::new(binary);
    if binary_path.is_absolute()
        || binary.contains('/')
        || binary.contains('\\')
        || binary_path.components().count() > 1
    {
        return is_executable_file(binary_path).then(|| binary_path.to_path_buf());
    }

    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        for candidate in candidate_binary_names(binary) {
            let candidate_path = dir.join(candidate);
            if is_executable_file(&candidate_path) {
                return Some(candidate_path);
            }
        }
    }

    None
}

fn shell_like_split(input: &str) -> Vec<String> {
    #[derive(Clone, Copy)]
    enum Quote {
        None,
        Single,
        Double,
    }

    let mut out = Vec::<String>::new();
    let mut buf = String::new();
    let mut chars = input.chars().peekable();
    let mut quote = Quote::None;

    while let Some(ch) = chars.next() {
        match quote {
            Quote::None => match ch {
                '\'' => quote = Quote::Single,
                '"' => quote = Quote::Double,
                '\\' => {
                    if let Some(next) = chars.next() {
                        buf.push(next);
                    }
                }
                c if c.is_whitespace() => {
                    if !buf.is_empty() {
                        out.push(std::mem::take(&mut buf));
                    }
                }
                _ => buf.push(ch),
            },
            Quote::Single => {
                if ch == '\'' {
                    quote = Quote::None;
                } else {
                    buf.push(ch);
                }
            }
            Quote::Double => {
                if ch == '"' {
                    quote = Quote::None;
                } else if ch == '\\' {
                    if let Some(next) = chars.next() {
                        buf.push(next);
                    }
                } else {
                    buf.push(ch);
                }
            }
        }
    }

    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

fn is_env_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap_or('_');
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

#[cfg(windows)]
fn candidate_binary_names(binary: &str) -> Vec<String> {
    let mut names = vec![binary.to_string()];
    if Path::new(binary).extension().is_none() {
        let ext_var = std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD;.VBS;.JS;.PS1".to_string());
        for ext in ext_var.split(';').map(str::trim).filter(|e| !e.is_empty()) {
            names.push(format!("{}{}", binary, ext));
        }
    }
    names
}

#[cfg(not(windows))]
fn candidate_binary_names(binary: &str) -> Vec<String> {
    vec![binary.to_string()]
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            return (meta.permissions().mode() & 0o111) != 0;
        }
        false
    }

    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_quoted_command_binary() {
        let cmd = "\"/Applications/My App/bin/mcp\" --stdio";
        let binary = extract_command_binary(cmd);
        assert_eq!(binary.as_deref(), Some("/Applications/My App/bin/mcp"));
    }

    #[test]
    fn extracts_binary_after_env_assignments() {
        let cmd = "FOO=1 BAR=2 npx -y @modelcontextprotocol/server-github";
        let binary = extract_command_binary(cmd);
        assert_eq!(binary.as_deref(), Some("npx"));
    }

    #[test]
    fn resolves_git_in_path() {
        assert!(resolve_binary_in_path("git").is_some());
    }
}

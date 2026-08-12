use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use serde_json::{Value, json};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};

use super::AppServerMessage;

#[derive(Debug, Error)]
pub enum AppServerError {
    #[error("failed to start Codex app-server: {0}")]
    Start(#[source] std::io::Error),
    #[error("Codex app-server did not expose {0}")]
    MissingPipe(&'static str),
    #[error("Codex app-server I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Codex app-server returned invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Codex app-server request timed out")]
    Timeout,
    #[error("Codex app-server closed the connection")]
    Closed,
    #[error("Codex app-server error {code}: {message}")]
    Rpc { code: i64, message: String },
}

pub struct AppServerClient {
    child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    next_id: i64,
}

impl AppServerClient {
    pub async fn start(configured_path: &str) -> Result<Self, AppServerError> {
        let program =
            resolve_codex_program(configured_path).unwrap_or_else(|| PathBuf::from("codex"));
        let mut child = Command::new(program)
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(AppServerError::Start)?;
        let stdin = child.stdin.take().ok_or(AppServerError::MissingPipe("stdin"))?;
        let stdout = child.stdout.take().ok_or(AppServerError::MissingPipe("stdout"))?;
        let mut client = Self { child, stdin, lines: BufReader::new(stdout).lines(), next_id: 1 };
        client.initialize().await?;
        Ok(client)
    }

    async fn initialize(&mut self) -> Result<(), AppServerError> {
        self.request("initialize", json!({
            "clientInfo": { "name": "ai_quota_trends", "title": "AI Quota Trends", "version": env!("CARGO_PKG_VERSION") },
            "capabilities": { "experimentalApi": true }
        })).await?;
        self.send(json!({ "method": "initialized", "params": {} })).await
    }

    pub async fn read_account(&mut self) -> Result<Value, AppServerError> {
        self.request("account/read", json!({ "refreshToken": false })).await
    }

    pub async fn read_rate_limits(&mut self) -> Result<Value, AppServerError> {
        self.request("account/rateLimits/read", Value::Null).await
    }

    pub async fn read_account_usage(&mut self) -> Result<Value, AppServerError> {
        self.request("account/usage/read", Value::Null).await
    }

    pub async fn next_message(&mut self) -> Result<AppServerMessage, AppServerError> {
        self.read_message().await
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, AppServerError> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "id": id, "method": method, "params": params })).await?;
        timeout(Duration::from_secs(15), async {
            loop {
                let message = self.read_message().await?;
                if message.id != Some(id) {
                    continue;
                }
                if let Some(error) = message.error {
                    return Err(AppServerError::Rpc { code: error.code, message: error.message });
                }
                return message.result.ok_or(AppServerError::Closed);
            }
        })
        .await
        .map_err(|_| AppServerError::Timeout)?
    }

    async fn send(&mut self, value: Value) -> Result<(), AppServerError> {
        let mut bytes = serde_json::to_vec(&value)?;
        bytes.push(b'\n');
        self.stdin.write_all(&bytes).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<AppServerMessage, AppServerError> {
        let line = self.lines.next_line().await?.ok_or(AppServerError::Closed)?;
        serde_json::from_str(&line).map_err(AppServerError::Json)
    }
}

pub fn resolve_codex_program(configured_path: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH");
    let home = env::var_os("HOME");
    resolve_codex_program_from_env(configured_path, path.as_deref(), home.as_deref().map(Path::new))
}

fn resolve_codex_program_from_env(
    configured_path: &str,
    path: Option<&OsStr>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    let configured_path = configured_path.trim();
    if !configured_path.is_empty() {
        return Some(expand_home(configured_path, home));
    }

    if let Some(path) = path {
        for directory in
            env::split_paths(path).filter(|directory| !directory.as_os_str().is_empty())
        {
            let candidate = directory.join("codex");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    let mut candidates = home
        .map(Path::to_path_buf)
        .into_iter()
        .flat_map(|home| [home.join(".volta/bin/codex"), home.join(".local/bin/codex")])
        .collect::<Vec<_>>();
    candidates
        .extend([PathBuf::from("/opt/homebrew/bin/codex"), PathBuf::from("/usr/local/bin/codex")]);
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn expand_home(path: &str, home: Option<&Path>) -> PathBuf {
    if path == "~" {
        return home.map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(path));
    }
    if let Some(relative) = path.strip_prefix("~/")
        && let Some(home) = home
    {
        return home.join(relative);
    }
    PathBuf::from(path)
}

impl Drop for AppServerClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, fs, path::Path};

    use tempfile::tempdir;

    use super::{resolve_codex_program, resolve_codex_program_from_env};

    fn write_executable(path: &Path) {
        fs::write(path, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).unwrap();
        }
    }

    #[test]
    fn explicit_legacy_override_wins_with_an_empty_path() {
        let temp = tempdir().unwrap();
        let configured = temp.path().join("custom-codex");
        write_executable(&configured);

        assert_eq!(
            resolve_codex_program_from_env(
                configured.to_str().unwrap(),
                Some(OsStr::new("")),
                Some(temp.path()),
            ),
            Some(configured)
        );
    }

    #[test]
    fn empty_path_discovers_the_home_codex_install() {
        let temp = tempdir().unwrap();
        let configured = temp.path().join(".volta/bin/codex");
        fs::create_dir_all(configured.parent().unwrap()).unwrap();
        write_executable(&configured);

        assert_eq!(
            resolve_codex_program_from_env("", Some(OsStr::new("")), Some(temp.path())),
            Some(configured)
        );
    }

    #[test]
    fn resolver_returns_only_codex_named_programs() {
        assert!(resolve_codex_program("").is_none_or(|path| {
            path.file_name().and_then(|name| name.to_str()) == Some("codex")
        }));
    }
}

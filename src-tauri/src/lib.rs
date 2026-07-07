use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{env, path::Path, process::Command, sync::OnceLock};

#[cfg(not(target_os = "windows"))]
use std::{fs, path::PathBuf};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

static CLI_INVOCATION: OnceLock<Result<CliInvocation, String>> = OnceLock::new();
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug)]
struct CliInvocation {
    program: String,
    prefix_args: Vec<String>,
    display_path: String,
    path_env: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CliCommandResult {
    ok: bool,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    command_display: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CliStatusOverview {
    auto_switch_enabled: bool,
    auto_switch_label: String,
    service_status: String,
    threshold_5h_percent: Option<u8>,
    threshold_weekly_percent: Option<u8>,
    usage_mode: String,
    account_mode: String,
    raw_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CliAccountOverview {
    index: String,
    label: String,
    email: Option<String>,
    selector: String,
    plan: String,
    usage_5h: String,
    usage_weekly: String,
    last_activity: String,
    is_active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CliOverview {
    version: String,
    cli_path: String,
    status: CliStatusOverview,
    accounts: Vec<CliAccountOverview>,
    supported_commands: Vec<String>,
    raw_help: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    mode: Option<String>,
    path: Option<String>,
    alias: Option<String>,
    payload: Option<String>,
}

fn cli_invocation(cli_path: Option<String>) -> Result<CliInvocation, String> {
    if let Some(cli_path) = normalized_cli_path(cli_path) {
        return discover_cli_invocation_for_path(&cli_path);
    }

    CLI_INVOCATION
        .get_or_init(|| {
            if let Some(cli_path) = env::var("CODEX_AUTH_CLI")
                .ok()
                .and_then(|value| normalized_cli_path(Some(value)))
            {
                discover_cli_invocation_for_path(&cli_path)
            } else {
                discover_cli_invocation()
            }
        })
        .as_ref()
        .cloned()
        .map_err(Clone::clone)
}

fn configure_hidden_process(_command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        _command.creation_flags(CREATE_NO_WINDOW);
    }
}

fn normalized_cli_path(cli_path: Option<String>) -> Option<String> {
    cli_path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_cli_invocation(mut invocation: CliInvocation) -> Result<CliInvocation, String> {
    let mut command = Command::new(&invocation.program);
    configure_hidden_process(&mut command);
    command.args(&invocation.prefix_args);
    if let Some(path_env) = invocation.path_env.as_deref() {
        command.env("PATH", path_env);
    }
    command.arg("--version");

    let output = command.output().map_err(|error| {
        format!(
            "Failed to run codex-auth at {}: {error}",
            invocation.display_path
        )
    })?;

    if !output.status.success() {
        return Err(format!(
            "Resolved codex-auth at {}, but it did not run successfully.\n\n{}",
            invocation.display_path,
            preferred_output_message(
                &String::from_utf8_lossy(&output.stdout),
                &String::from_utf8_lossy(&output.stderr),
            )
        ));
    }

    invocation.display_path = invocation.display_path.trim().to_string();
    Ok(invocation)
}

fn discover_cli_invocation() -> Result<CliInvocation, String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(invocation) = resolve_windows_cli_through_powershell()? {
            return validate_cli_invocation(invocation);
        }

        Err("Could not resolve codex-auth. Install @loongphy/codex-auth, then restart the app or paste the full codex-auth path in Settings.".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        discover_unix_cli_invocation()
    }
}

#[cfg(target_os = "windows")]
fn resolve_windows_cli_through_powershell() -> Result<Option<CliInvocation>, String> {
    let resolver_script = r#"
$pathValues = @(
  [Environment]::GetEnvironmentVariable('Path', 'Process'),
  [Environment]::GetEnvironmentVariable('Path', 'User'),
  [Environment]::GetEnvironmentVariable('Path', 'Machine')
)

$directPaths = @(
  [Environment]::GetEnvironmentVariable('NVM_SYMLINK', 'Process'),
  [Environment]::GetEnvironmentVariable('NVM_SYMLINK', 'User'),
  [Environment]::GetEnvironmentVariable('NVM_SYMLINK', 'Machine'),
  [Environment]::GetEnvironmentVariable('APPDATA', 'Process') + '\npm',
  [Environment]::GetEnvironmentVariable('APPDATA', 'User') + '\npm',
  [Environment]::GetEnvironmentVariable('LOCALAPPDATA', 'Process') + '\Microsoft\WinGet\Links',
  [Environment]::GetEnvironmentVariable('LOCALAPPDATA', 'User') + '\Microsoft\WinGet\Links',
  'C:\nvm4w\nodejs',
  'C:\Program Files\nodejs',
  'C:\Program Files (x86)\nodejs'
)

$parts = New-Object System.Collections.Generic.List[string]
foreach ($value in $pathValues) {
  if ([string]::IsNullOrWhiteSpace($value)) { continue }
  foreach ($part in $value -split ';') {
    if ([string]::IsNullOrWhiteSpace($part)) { continue }
    $expanded = [Environment]::ExpandEnvironmentVariables($part.Trim())
    if (-not [string]::IsNullOrWhiteSpace($expanded)) { $parts.Add($expanded) }
  }
}

foreach ($path in $directPaths) {
  if ([string]::IsNullOrWhiteSpace($path)) { continue }
  $expanded = [Environment]::ExpandEnvironmentVariables($path.Trim())
  if (-not [string]::IsNullOrWhiteSpace($expanded)) { $parts.Add($expanded) }
}

$unique = New-Object System.Collections.Generic.List[string]
foreach ($part in $parts) {
  if ($unique -notcontains $part) { $unique.Add($part) }
}

$env:Path = $unique -join ';'
$resolved = (Get-Command codex-auth -ErrorAction Stop).Source
"__CODEX_AUTH_PATH__$resolved"
"__CODEX_AUTH_ENV__$env:Path"
"#;

    let mut command = Command::new("powershell");
    configure_hidden_process(&mut command);
    let output = command
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            resolver_script,
        ])
        .output()
        .map_err(|error| format!("Failed to locate codex-auth through PowerShell: {error}"))?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let script_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__CODEX_AUTH_PATH__"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let path_env = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__CODEX_AUTH_ENV__"))
        .unwrap_or_default()
        .trim()
        .to_string();

    if script_path.is_empty() {
        return Err(format!(
            "PowerShell resolved an empty codex-auth path. {}",
            preferred_output_message(
                &String::from_utf8_lossy(&output.stdout),
                &String::from_utf8_lossy(&output.stderr),
            )
        ));
    }

    let path_env = if path_env.is_empty() {
        None
    } else {
        Some(path_env)
    };

    Ok(Some(windows_cli_invocation_for_resolved_path(
        &script_path,
        path_env,
    )))
}

#[cfg(target_os = "windows")]
fn windows_cli_invocation_for_resolved_path(
    cli_path: &str,
    path_env: Option<String>,
) -> CliInvocation {
    let normalized_path = cli_path.to_string();
    let lower_path = normalized_path.to_ascii_lowercase();

    if lower_path.ends_with(".ps1") {
        return CliInvocation {
            program: "powershell".to_string(),
            prefix_args: vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
                normalized_path.clone(),
            ],
            display_path: normalized_path,
            path_env,
        };
    }

    if lower_path.ends_with(".cmd") || lower_path.ends_with(".bat") {
        return CliInvocation {
            program: "cmd".to_string(),
            prefix_args: vec!["/C".to_string(), normalized_path.clone()],
            display_path: normalized_path,
            path_env,
        };
    }

    CliInvocation {
        program: normalized_path.clone(),
        prefix_args: Vec::new(),
        display_path: normalized_path,
        path_env,
    }
}

fn discover_cli_invocation_for_path(cli_path: &str) -> Result<CliInvocation, String> {
    #[cfg(target_os = "windows")]
    {
        validate_cli_invocation(windows_cli_invocation_for_resolved_path(cli_path, None))
    }

    #[cfg(not(target_os = "windows"))]
    {
        unix_cli_invocation_for_candidate(cli_path, None)
    }
}

#[cfg(not(target_os = "windows"))]
fn discover_unix_cli_invocation() -> Result<CliInvocation, String> {
    if let Ok(invocation) = unix_cli_invocation_for_candidate("codex-auth", None) {
        return Ok(invocation);
    }

    for shell_path in ["/bin/zsh", "/bin/bash", "/bin/sh"] {
        if let Some((cli_path, path_env)) = resolve_cli_through_shell(shell_path) {
            if let Ok(invocation) = unix_cli_invocation_for_candidate(&cli_path, Some(path_env)) {
                return Ok(invocation);
            }
        }
    }

    for candidate in fallback_unix_cli_paths() {
        if let Ok(invocation) =
            unix_cli_invocation_for_candidate(&candidate.to_string_lossy(), None)
        {
            return Ok(invocation);
        }
    }

    Err(format!(
        "Could not resolve codex-auth. GUI apps on macOS do not inherit your terminal PATH.\n\nCurrent PATH: {}\n\nInstall @loongphy/codex-auth, then either launch from a shell that can run codex-auth or paste the full codex-auth path in Settings.",
        env::var("PATH").unwrap_or_else(|_| "<unset>".to_string())
    ))
}

#[cfg(not(target_os = "windows"))]
fn resolve_cli_through_shell(shell_path: &str) -> Option<(String, String)> {
    let output = Command::new(shell_path)
        .args([
            "-ilc",
            "resolved=$(command -v codex-auth) && printf '__CODEX_AUTH_PATH__%s\\n__CODEX_AUTH_ENV__%s\\n' \"$resolved\" \"$PATH\"",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let cli_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__CODEX_AUTH_PATH__"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let path_env = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__CODEX_AUTH_ENV__"))
        .unwrap_or_default()
        .trim()
        .to_string();

    if cli_path.is_empty() {
        None
    } else {
        Some((cli_path, path_env))
    }
}

#[cfg(not(target_os = "windows"))]
fn fallback_unix_cli_paths() -> Vec<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin/codex-auth"),
        PathBuf::from("/usr/local/bin/codex-auth"),
        PathBuf::from("/usr/bin/codex-auth"),
    ];

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".npm-global/bin/codex-auth"));
        candidates.push(home.join(".local/bin/codex-auth"));
        candidates.push(home.join(".bun/bin/codex-auth"));

        let nvm_versions = home.join(".nvm/versions/node");
        if let Ok(entries) = fs::read_dir(nvm_versions) {
            let mut nvm_candidates = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("bin/codex-auth"))
                .collect::<Vec<_>>();
            nvm_candidates.sort();
            nvm_candidates.reverse();
            candidates.extend(nvm_candidates);
        }
    }

    candidates
}

#[cfg(not(target_os = "windows"))]
fn unix_cli_invocation_for_candidate(
    candidate: &str,
    discovered_path_env: Option<String>,
) -> Result<CliInvocation, String> {
    let path_env = unix_path_env_for_candidate(candidate, discovered_path_env);

    validate_cli_invocation(CliInvocation {
        program: candidate.to_string(),
        prefix_args: Vec::new(),
        display_path: candidate.to_string(),
        path_env,
    })
}

#[cfg(not(target_os = "windows"))]
fn unix_path_env_for_candidate(
    candidate: &str,
    discovered_path_env: Option<String>,
) -> Option<String> {
    let mut path_parts = Vec::new();

    if let Some(path_env) = discovered_path_env {
        push_path_parts(&mut path_parts, &path_env);
    }

    if let Some(parent) = Path::new(candidate)
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        path_parts.push(parent.to_string_lossy().to_string());
    }

    push_path_parts(
        &mut path_parts,
        &env::var("PATH").unwrap_or_else(|_| {
            "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin".to_string()
        }),
    );

    for fallback in [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ] {
        path_parts.push(fallback.to_string());
    }

    let mut unique_parts = Vec::new();
    for part in path_parts {
        if !part.is_empty() && !unique_parts.contains(&part) {
            unique_parts.push(part);
        }
    }

    if unique_parts.is_empty() {
        None
    } else {
        Some(unique_parts.join(":"))
    }
}

#[cfg(not(target_os = "windows"))]
fn push_path_parts(parts: &mut Vec<String>, path_env: &str) {
    parts.extend(
        env::split_paths(path_env)
            .map(|path| path.to_string_lossy().to_string())
            .filter(|path| !path.is_empty()),
    );
}

fn preferred_output_message(stdout: &str, stderr: &str) -> String {
    let trimmed_stderr = stderr.trim();
    if !trimmed_stderr.is_empty() {
        return trimmed_stderr.to_string();
    }

    let trimmed_stdout = stdout.trim();
    if !trimmed_stdout.is_empty() {
        return trimmed_stdout.to_string();
    }

    "No output was produced.".to_string()
}

fn execute_codex_auth(
    args: &[String],
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    let invocation = cli_invocation(cli_path)?;
    let mut command = Command::new(&invocation.program);
    configure_hidden_process(&mut command);
    if let Some(path_env) = invocation.path_env.as_deref() {
        command.env("PATH", path_env);
    }
    command.args(&invocation.prefix_args);
    command.args(args);

    let output = command
        .output()
        .map_err(|error| format!("Failed to run codex-auth: {error}"))?;

    Ok(CliCommandResult {
        ok: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
        command_display: format!("codex-auth {}", args.join(" ")).trim().to_string(),
    })
}

fn ensure_success(context: &str, result: CliCommandResult) -> Result<CliCommandResult, String> {
    if result.ok {
        Ok(result)
    } else {
        Err(format!(
            "{context} failed.\n\n{}",
            preferred_output_message(&result.stdout, &result.stderr)
        ))
    }
}

fn push_refresh_mode(args: &mut Vec<String>, refresh_mode: Option<String>) {
    match refresh_mode.as_deref().map(str::trim) {
        Some("api") | Some("--api") => args.push("--api".to_string()),
        Some("skip-api") | Some("--skip-api") | Some("local") => {
            args.push("--skip-api".to_string())
        }
        _ => {}
    }
}

fn parse_status_output(output: &str) -> CliStatusOverview {
    let threshold_regex = Regex::new(r"5h<(\d+)%\s*,\s*weekly<(\d+)%").expect("valid regex");
    let mut auto_switch_label = "Unknown".to_string();
    let mut service_status = "unknown".to_string();
    let mut threshold_5h_percent = None;
    let mut threshold_weekly_percent = None;
    let mut usage_mode = "unknown".to_string();
    let mut account_mode = "unknown".to_string();

    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Some((key, value)) = line.split_once(':') {
            let normalized_key = key.trim().to_ascii_lowercase();
            let normalized_value = value.trim();

            match normalized_key.as_str() {
                "auto-switch" => auto_switch_label = normalized_value.to_string(),
                "service" => service_status = normalized_value.to_string(),
                "thresholds" => {
                    if let Some(captures) = threshold_regex.captures(normalized_value) {
                        threshold_5h_percent = captures
                            .get(1)
                            .and_then(|value| value.as_str().parse::<u8>().ok());
                        threshold_weekly_percent = captures
                            .get(2)
                            .and_then(|value| value.as_str().parse::<u8>().ok());
                    }
                }
                "usage" => usage_mode = normalized_value.to_string(),
                "account" => account_mode = normalized_value.to_string(),
                _ => {}
            }
        }
    }

    let auto_switch_enabled = auto_switch_label.eq_ignore_ascii_case("on");

    CliStatusOverview {
        auto_switch_enabled,
        auto_switch_label,
        service_status,
        threshold_5h_percent,
        threshold_weekly_percent,
        usage_mode,
        account_mode,
        raw_status: output.trim().to_string(),
    }
}

fn parse_email(text: &str) -> Option<String> {
    let email_regex =
        Regex::new(r"(?i)[a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,}").expect("valid regex");
    email_regex
        .find(text)
        .map(|value| value.as_str().to_string())
}

fn parse_list_output(output: &str) -> Vec<CliAccountOverview> {
    let row_regex =
        Regex::new(r"^(?P<active>\*)?\s*(?P<index>\d{2})\s+(?P<rest>.+)$").expect("valid regex");
    let split_regex = Regex::new(r"\s{2,}").expect("valid regex");
    let mut accounts = Vec::new();

    for line in output.lines().map(str::trim_end) {
        if line.trim().is_empty() || line.trim_start().starts_with('-') {
            continue;
        }

        if line.contains("ACCOUNT") && line.contains("PLAN") && line.contains("LAST ACTIVITY") {
            continue;
        }

        let Some(captures) = row_regex.captures(line) else {
            continue;
        };

        let Some(index_match) = captures.name("index") else {
            continue;
        };

        let Some(rest_match) = captures.name("rest") else {
            continue;
        };

        let mut columns = split_regex
            .split(rest_match.as_str().trim())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();

        if columns.len() < 5 {
            continue;
        }

        let last_activity = columns.pop().unwrap_or_default().to_string();
        let usage_weekly = columns.pop().unwrap_or_default().to_string();
        let usage_5h = columns.pop().unwrap_or_default().to_string();
        let plan = columns.pop().unwrap_or_default().to_string();
        let label = columns.join(" ");
        let email = parse_email(&label);
        let selector = email.clone().unwrap_or_else(|| label.clone());

        accounts.push(CliAccountOverview {
            index: index_match.as_str().to_string(),
            label,
            email,
            selector,
            plan,
            usage_5h,
            usage_weekly,
            last_activity,
            is_active: captures.name("active").is_some(),
        });
    }

    accounts
}

fn parse_supported_commands(help_output: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_commands = false;

    for line in help_output.lines() {
        let trimmed = line.trim();

        if trimmed == "Commands:" {
            in_commands = true;
            continue;
        }

        if !in_commands {
            continue;
        }

        if trimmed.is_empty() || trimmed == "Notes:" {
            break;
        }

        if !line.starts_with("  ") || line.starts_with("    ") {
            continue;
        }

        let token = trimmed
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches(',');
        if !token.is_empty() {
            commands.push(token.to_string());
        }
    }

    commands
}

fn refresh_mode_label(refresh_mode: Option<&str>) -> String {
    match refresh_mode.map(str::trim) {
        Some("skip-api") | Some("--skip-api") | Some("local") => "local".to_string(),
        Some("api") | Some("--api") => "api".to_string(),
        _ => "api".to_string(),
    }
}

fn status_overview_from_list(list_output: &str, refresh_mode: Option<&str>) -> CliStatusOverview {
    let active_account = parse_list_output(list_output)
        .into_iter()
        .find(|account| account.is_active);
    let account_mode = active_account
        .as_ref()
        .map(|account| account.label.clone())
        .unwrap_or_else(|| "none".to_string());

    CliStatusOverview {
        auto_switch_enabled: false,
        auto_switch_label: "Not supported by codex-auth 0.3".to_string(),
        service_status: "ready".to_string(),
        threshold_5h_percent: None,
        threshold_weekly_percent: None,
        usage_mode: refresh_mode_label(refresh_mode),
        account_mode,
        raw_status: list_output.trim().to_string(),
    }
}

fn is_codex_auth_executable(token: &str) -> bool {
    let normalized = token.trim_matches('"').trim_matches('\'');
    let Some(file_name) = Path::new(normalized).file_name() else {
        return false;
    };

    let file_name = file_name.to_string_lossy().to_ascii_lowercase();
    matches!(
        file_name.as_str(),
        "codex-auth" | "codex-auth.exe" | "codex-auth.cmd" | "codex-auth.bat" | "codex-auth.ps1"
    )
}

fn parse_raw_command(command: &str) -> Result<Vec<String>, String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("Enter a codex-auth command or arguments first.".to_string());
    }

    let mut args = shlex::split(trimmed).ok_or_else(|| {
        "Unable to parse the command. Quote file paths or aliases that contain spaces.".to_string()
    })?;

    if args
        .first()
        .is_some_and(|token| is_codex_auth_executable(token))
    {
        args.remove(0);
    }

    Ok(args)
}

async fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| format!("Background task failed: {error}"))?
}

#[tauri::command]
async fn get_cli_overview(
    refresh_mode: Option<String>,
    cli_path: Option<String>,
) -> Result<CliOverview, String> {
    run_blocking(move || {
        let invocation = cli_invocation(cli_path.clone())?;
        let version_result = ensure_success(
            "Loading codex-auth version",
            execute_codex_auth(&[String::from("--version")], cli_path.clone())?,
        )?;
        let help_result = ensure_success(
            "Loading codex-auth help",
            execute_codex_auth(&[String::from("--help")], cli_path.clone())?,
        )?;
        let supported_commands = parse_supported_commands(&help_result.stdout);

        let mut list_args = vec![String::from("list")];
        push_refresh_mode(&mut list_args, refresh_mode.clone());
        let list_result = ensure_success(
            "Loading codex-auth accounts",
            execute_codex_auth(&list_args, cli_path.clone())?,
        )?;
        let status = if supported_commands.iter().any(|command| command == "status") {
            let status_result = ensure_success(
                "Loading codex-auth status",
                execute_codex_auth(&[String::from("status")], cli_path.clone())?,
            )?;
            parse_status_output(&status_result.stdout)
        } else {
            status_overview_from_list(&list_result.stdout, refresh_mode.as_deref())
        };

        Ok(CliOverview {
            version: version_result.stdout.trim().to_string(),
            cli_path: invocation.display_path,
            status,
            accounts: parse_list_output(&list_result.stdout),
            supported_commands,
            raw_help: help_result.stdout,
        })
    })
    .await
}

#[tauri::command]
async fn switch_account(
    account_id: Option<String>,
    query: Option<String>,
    refresh_mode: Option<String>,
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    let selector = account_id
        .or(query)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Pick an account before switching.".to_string())?;

    run_blocking(move || {
        let mut args = vec![String::from("switch")];
        push_refresh_mode(&mut args, refresh_mode);
        args.push(selector);
        ensure_success("Switching accounts", execute_codex_auth(&args, cli_path)?)
    })
    .await
}

#[tauri::command]
async fn remove_accounts(
    account_ids: Option<Vec<String>>,
    queries: Option<Vec<String>>,
    all: Option<bool>,
    refresh_mode: Option<String>,
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    let remove_all = all.unwrap_or(false);
    let selectors = account_ids
        .or(queries)
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    run_blocking(move || {
        if remove_all {
            return ensure_success(
                "Removing all accounts",
                execute_codex_auth(
                    &[String::from("remove"), String::from("--all")],
                    cli_path.clone(),
                )?,
            );
        }

        if selectors.is_empty() {
            return Err("Select at least one account to remove.".to_string());
        }

        let mut stdout_chunks = Vec::new();
        let mut stderr_chunks = Vec::new();

        for selector in selectors {
            let mut args = vec![String::from("remove")];
            push_refresh_mode(&mut args, refresh_mode.clone());
            args.push(selector);
            let result = ensure_success(
                "Removing account",
                execute_codex_auth(&args, cli_path.clone())?,
            )?;

            if !result.stdout.trim().is_empty() {
                stdout_chunks.push(result.stdout.trim().to_string());
            }

            if !result.stderr.trim().is_empty() {
                stderr_chunks.push(result.stderr.trim().to_string());
            }
        }

        Ok(CliCommandResult {
            ok: true,
            stdout: stdout_chunks.join("\n"),
            stderr: stderr_chunks.join("\n"),
            exit_code: Some(0),
            command_display: "codex-auth remove <selected accounts>".to_string(),
        })
    })
    .await
}

#[tauri::command]
async fn start_login(
    device_auth: Option<bool>,
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    run_blocking(move || {
        let mut args = vec![String::from("login")];
        if device_auth.unwrap_or(false) {
            args.push(String::from("--device-auth"));
        }

        ensure_success("Starting login flow", execute_codex_auth(&args, cli_path)?)
    })
    .await
}

#[tauri::command]
async fn run_import(
    request: ImportRequest,
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    run_blocking(move || {
        let mode = request
            .mode
            .unwrap_or_else(|| "file".to_string())
            .trim()
            .to_string();
        let path = request
            .path
            .or(request.payload)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let alias = request
            .alias
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let mut args = vec![String::from("import")];

        match mode.as_str() {
            "cpa" => {
                args.push(String::from("--cpa"));
                if let Some(path) = path {
                    args.push(path);
                }
            }
            "purge" => {
                args.push(String::from("--purge"));
                if let Some(path) = path {
                    args.push(path);
                }
            }
            _ => {
                let Some(path) = path else {
                    return Err("Choose a file or directory to import.".to_string());
                };
                args.push(path);
            }
        }

        if let Some(alias) = alias {
            args.push(String::from("--alias"));
            args.push(alias);
        }

        ensure_success("Running import", execute_codex_auth(&args, cli_path)?)
    })
    .await
}

#[tauri::command]
async fn set_auto_switch(
    enabled: bool,
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    run_blocking(move || {
        let args = vec![
            String::from("config"),
            String::from("auto"),
            if enabled { "enable" } else { "disable" }.to_string(),
        ];

        ensure_success("Updating auto-switch", execute_codex_auth(&args, cli_path)?)
    })
    .await
}

#[tauri::command]
async fn set_auto_switch_thresholds(
    threshold_5h_percent: Option<u8>,
    threshold_weekly_percent: Option<u8>,
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    run_blocking(move || {
        if threshold_5h_percent.is_none() && threshold_weekly_percent.is_none() {
            return Err("Provide at least one threshold value.".to_string());
        }

        let mut args = vec![String::from("config"), String::from("auto")];

        if let Some(value) = threshold_5h_percent {
            args.push(String::from("--5h"));
            args.push(value.to_string());
        }

        if let Some(value) = threshold_weekly_percent {
            args.push(String::from("--weekly"));
            args.push(value.to_string());
        }

        ensure_success("Updating thresholds", execute_codex_auth(&args, cli_path)?)
    })
    .await
}

#[tauri::command]
async fn set_api_mode(
    enabled: Option<bool>,
    mode: Option<String>,
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    let resolved_enabled = enabled.or_else(|| match mode.as_deref().map(str::trim) {
        Some("api") => Some(true),
        Some("local") | Some("skip-api") | Some("chatgpt") => Some(false),
        _ => None,
    });

    let Some(resolved_enabled) = resolved_enabled else {
        return Err("Pick either API mode or local mode.".to_string());
    };

    run_blocking(move || {
        let args = vec![
            String::from("config"),
            String::from("api"),
            if resolved_enabled {
                "enable"
            } else {
                "disable"
            }
            .to_string(),
        ];

        ensure_success("Updating API mode", execute_codex_auth(&args, cli_path)?)
    })
    .await
}

#[tauri::command]
async fn clean_accounts(cli_path: Option<String>) -> Result<CliCommandResult, String> {
    run_blocking(move || {
        ensure_success(
            "Cleaning account files",
            execute_codex_auth(&[String::from("clean")], cli_path)?,
        )
    })
    .await
}

#[tauri::command]
async fn get_command_help(
    command: Option<String>,
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    run_blocking(move || {
        let args = if let Some(command) = command
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            vec![command, String::from("--help")]
        } else {
            vec![String::from("--help")]
        };

        ensure_success("Loading command help", execute_codex_auth(&args, cli_path)?)
    })
    .await
}

#[tauri::command]
async fn run_raw_command(
    command: String,
    cli_path: Option<String>,
) -> Result<CliCommandResult, String> {
    run_blocking(move || {
        let args = parse_raw_command(&command)?;
        execute_codex_auth(&args, cli_path)
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            get_cli_overview,
            switch_account,
            remove_accounts,
            start_login,
            run_import,
            set_auto_switch,
            set_auto_switch_thresholds,
            set_api_mode,
            clean_accounts,
            get_command_help,
            run_raw_command,
        ])
        .setup(|app| {
            let _ = app;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        parse_list_output, parse_raw_command, parse_status_output, parse_supported_commands,
        status_overview_from_list,
    };

    #[test]
    fn parses_status_output() {
        let parsed = parse_status_output(
            "auto-switch: OFF\nservice: stopped\nthresholds: 5h<10%, weekly<5%\nusage: api\naccount: api\n",
        );

        assert!(!parsed.auto_switch_enabled);
        assert_eq!(parsed.service_status, "stopped");
        assert_eq!(parsed.threshold_5h_percent, Some(10));
        assert_eq!(parsed.threshold_weekly_percent, Some(5));
        assert_eq!(parsed.usage_mode, "api");
        assert_eq!(parsed.account_mode, "api");
    }

    #[test]
    fn parses_list_output() {
        let parsed = parse_list_output(
            "     ACCOUNT                    PLAN  5H USAGE               WEEKLY USAGE            LAST ACTIVITY\n\
--------------------------------------------------------------------------------------------------\n\
* 01 active@example.com         Plus  91% (01:34 on 13 Jun)  12% (21:26 on 15 Jun)   Now\n\
  02 work alias@example.com     Pro   99% (01:27 on 13 Jun)  100% (20:37 on 19 Jun)  4m ago\n",
        );

        assert_eq!(parsed.len(), 2);
        assert!(parsed[0].is_active);
        assert_eq!(parsed[0].selector, "active@example.com");
        assert_eq!(parsed[1].email.as_deref(), Some("alias@example.com"));
        assert_eq!(parsed[1].plan, "Pro");
    }

    #[test]
    fn parses_v3_supported_commands() {
        let parsed = parse_supported_commands(
            "codex-auth 0.3.0-alpha.10\n\nCommands:\n  --help, -h\n      Show this help\n  list [--live] [--active] [--api|--skip-api]\n      List available accounts\n  config\n      Manage configuration\n\nNotes:\n  Run help.\n",
        );

        assert_eq!(
            parsed,
            vec![
                "--help".to_string(),
                "list".to_string(),
                "config".to_string()
            ]
        );
    }

    #[test]
    fn derives_v3_status_from_list_output() {
        let parsed = status_overview_from_list(
            "     ACCOUNT                    PLAN  5H           WEEKLY       LAST ACTIVITY\n\
-----------------------------------------------------------------------------\n\
  01 first@example.com          Plus  100%         100%         9d ago\n\
* 02 active@example.com         Plus  36% (20:32)  35% (18:19)  Now\n",
            Some("skip-api"),
        );

        assert_eq!(parsed.service_status, "ready");
        assert_eq!(parsed.usage_mode, "local");
        assert_eq!(parsed.account_mode, "active@example.com");
        assert_eq!(parsed.raw_status.lines().count(), 4);
    }

    #[test]
    fn parses_raw_arguments() {
        let parsed = parse_raw_command(r#"codex-auth import "C:\My Files\auth.json" --alias work"#)
            .expect("command should parse");

        assert_eq!(
            parsed,
            vec![
                "import".to_string(),
                r#"C:\My Files\auth.json"#.to_string(),
                "--alias".to_string(),
                "work".to_string()
            ]
        );
    }

    #[test]
    fn parses_raw_arguments_without_stripping_similar_command_names() {
        let parsed =
            parse_raw_command("codex-authentication status").expect("command should parse");

        assert_eq!(
            parsed,
            vec!["codex-authentication".to_string(), "status".to_string()]
        );
    }

    #[test]
    fn parses_raw_arguments_with_windows_shim_name() {
        let parsed =
            parse_raw_command(r#"codex-auth.ps1 list --skip-api"#).expect("command should parse");

        assert_eq!(parsed, vec!["list".to_string(), "--skip-api".to_string(),]);
    }
}

import { invoke } from '@tauri-apps/api/core'

export type RefreshMode = 'default' | 'api' | 'skip-api'
export type ImportMode = 'file' | 'cpa' | 'purge'

export type CliCommandResult = {
  ok: boolean
  stdout: string
  stderr: string
  exitCode: number | null
  commandDisplay: string
}

export type CliStatusOverview = {
  autoSwitchEnabled: boolean
  autoSwitchLabel: string
  serviceStatus: string
  threshold5hPercent: number | null
  thresholdWeeklyPercent: number | null
  usageMode: string
  accountMode: string
  rawStatus: string
}

export type CliAccountOverview = {
  index: string
  label: string
  email: string | null
  selector: string
  plan: string
  usage5h: string
  usageWeekly: string
  lastActivity: string
  isActive: boolean
}

export type CliOverview = {
  version: string
  cliPath: string
  status: CliStatusOverview
  accounts: CliAccountOverview[]
  supportedCommands: string[]
  rawHelp: string
}

export type ImportRequest = {
  mode: ImportMode
  path?: string
  alias?: string
}

export type CliOptions = {
  cliPath?: string
}

function normalizedCliPath(options?: CliOptions) {
  const cliPath = options?.cliPath?.trim()
  return cliPath || null
}

export async function getCliOverview(
  refreshMode: RefreshMode = 'default',
  options?: CliOptions,
) {
  return invoke<CliOverview>('get_cli_overview', {
    refreshMode: refreshMode === 'default' ? null : refreshMode,
    cliPath: normalizedCliPath(options),
  })
}

export async function switchAccount(
  selector: string,
  refreshMode: RefreshMode = 'default',
  options?: CliOptions,
) {
  return invoke<CliCommandResult>('switch_account', {
    accountId: selector,
    refreshMode: refreshMode === 'default' ? null : refreshMode,
    cliPath: normalizedCliPath(options),
  })
}

export async function switchAccountAndRelaunch(
  selector: string,
  refreshMode: RefreshMode = 'default',
  options?: CliOptions,
) {
  return invoke<CliCommandResult>('switch_account_and_relaunch', {
    accountId: selector,
    refreshMode: refreshMode === 'default' ? null : refreshMode,
    cliPath: normalizedCliPath(options),
  })
}

export async function stopCodexGui() {
  return invoke<void>('stop_codex_gui')
}

export async function startCodexGui() {
  return invoke<void>('start_codex_gui')
}

export async function openRepository() {
  return invoke<void>('open_repository')
}

export async function removeAccounts(
  selectors: string[],
  options?: { all?: boolean; refreshMode?: RefreshMode; cliPath?: string },
) {
  return invoke<CliCommandResult>('remove_accounts', {
    accountIds: selectors,
    all: options?.all ?? false,
    refreshMode:
      (options?.refreshMode ?? 'default') === 'default' ? null : options?.refreshMode,
    cliPath: normalizedCliPath(options),
  })
}

export async function startLogin(deviceAuth = false, options?: CliOptions) {
  return invoke<CliCommandResult>('start_login', {
    deviceAuth,
    cliPath: normalizedCliPath(options),
  })
}

export async function runImport(request: ImportRequest, options?: CliOptions) {
  return invoke<CliCommandResult>('run_import', {
    request,
    cliPath: normalizedCliPath(options),
  })
}

export async function setAutoSwitch(enabled: boolean, options?: CliOptions) {
  return invoke<CliCommandResult>('set_auto_switch', {
    enabled,
    cliPath: normalizedCliPath(options),
  })
}

export async function setAutoSwitchThresholds(
  threshold5hPercent?: number,
  thresholdWeeklyPercent?: number,
  options?: CliOptions,
) {
  return invoke<CliCommandResult>('set_auto_switch_thresholds', {
    threshold5hPercent: threshold5hPercent ?? null,
    thresholdWeeklyPercent: thresholdWeeklyPercent ?? null,
    cliPath: normalizedCliPath(options),
  })
}

export async function setApiMode(enabled: boolean, options?: CliOptions) {
  return invoke<CliCommandResult>('set_api_mode', {
    enabled,
    cliPath: normalizedCliPath(options),
  })
}

export async function cleanAccounts(options?: CliOptions) {
  return invoke<CliCommandResult>('clean_accounts', {
    cliPath: normalizedCliPath(options),
  })
}

export async function getCommandHelp(command?: string, options?: CliOptions) {
  return invoke<CliCommandResult>('get_command_help', {
    command: command?.trim() ? command.trim() : null,
    cliPath: normalizedCliPath(options),
  })
}

export async function runRawCommand(command: string, options?: CliOptions) {
  return invoke<CliCommandResult>('run_raw_command', {
    command,
    cliPath: normalizedCliPath(options),
  })
}

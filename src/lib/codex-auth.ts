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

export async function getCliOverview(refreshMode: RefreshMode = 'default') {
  return invoke<CliOverview>('get_cli_overview', {
    refreshMode: refreshMode === 'default' ? null : refreshMode,
  })
}

export async function switchAccount(selector: string, refreshMode: RefreshMode = 'default') {
  return invoke<CliCommandResult>('switch_account', {
    accountId: selector,
    refreshMode: refreshMode === 'default' ? null : refreshMode,
  })
}

export async function removeAccounts(
  selectors: string[],
  options?: { all?: boolean; refreshMode?: RefreshMode },
) {
  return invoke<CliCommandResult>('remove_accounts', {
    accountIds: selectors,
    all: options?.all ?? false,
    refreshMode:
      (options?.refreshMode ?? 'default') === 'default' ? null : options?.refreshMode,
  })
}

export async function startLogin(deviceAuth = false) {
  return invoke<CliCommandResult>('start_login', { deviceAuth })
}

export async function runImport(request: ImportRequest) {
  return invoke<CliCommandResult>('run_import', { request })
}

export async function setAutoSwitch(enabled: boolean) {
  return invoke<CliCommandResult>('set_auto_switch', { enabled })
}

export async function setAutoSwitchThresholds(
  threshold5hPercent?: number,
  thresholdWeeklyPercent?: number,
) {
  return invoke<CliCommandResult>('set_auto_switch_thresholds', {
    threshold5hPercent: threshold5hPercent ?? null,
    thresholdWeeklyPercent: thresholdWeeklyPercent ?? null,
  })
}

export async function setApiMode(enabled: boolean) {
  return invoke<CliCommandResult>('set_api_mode', { enabled })
}

export async function cleanAccounts() {
  return invoke<CliCommandResult>('clean_accounts')
}

export async function getCommandHelp(command?: string) {
  return invoke<CliCommandResult>('get_command_help', {
    command: command?.trim() ? command.trim() : null,
  })
}

export async function runRawCommand(command: string) {
  return invoke<CliCommandResult>('run_raw_command', { command })
}

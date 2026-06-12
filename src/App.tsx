import {
  ArrowRightLeft,
  Bot,
  BrushCleaning,
  FolderInput,
  Gauge,
  LogIn,
  RefreshCw,
  ShieldCheck,
  TerminalSquare,
  Trash2,
} from 'lucide-react'
import { useDeferredValue, useEffect, useState, useTransition } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import {
  cleanAccounts,
  getCliOverview,
  removeAccounts,
  runImport,
  runRawCommand,
  setApiMode,
  setAutoSwitch,
  setAutoSwitchThresholds,
  startLogin,
  switchAccount,
  type CliAccountOverview,
  type CliCommandResult,
  type ImportMode,
  type RefreshMode,
} from '@/lib/codex-auth'
import { cn } from '@/lib/utils'

type PageId = 'accounts' | 'add' | 'settings' | 'console'

type CommandLogEntry = {
  id: number
  title: string
  result: CliCommandResult
  createdAt: string
}

const refreshModeOptions: Array<{ label: string; value: RefreshMode }> = [
  { label: 'CLI default', value: 'default' },
  { label: 'API refresh', value: 'api' },
  { label: 'Local refresh', value: 'skip-api' },
]

const importModeOptions: Array<{ label: string; value: ImportMode }> = [
  { label: 'Auth file/folder', value: 'file' },
  { label: 'CPA JSON', value: 'cpa' },
  { label: 'Rebuild registry', value: 'purge' },
]
const overviewRefreshIntervalMs = 30_000

const pages: Array<{
  id: PageId
  label: string
  icon: typeof ArrowRightLeft
}> = [
  { id: 'accounts', label: 'Accounts', icon: ArrowRightLeft },
  { id: 'add', label: 'Add', icon: FolderInput },
  { id: 'settings', label: 'Settings', icon: Bot },
  { id: 'console', label: 'Console', icon: TerminalSquare },
]

function pageFromHash(hash: string): PageId {
  const normalized = hash.replace(/^#/, '')
  return pages.some((page) => page.id === normalized) ? (normalized as PageId) : 'accounts'
}

function App() {
  const queryClient = useQueryClient()
  const [, startTransition] = useTransition()
  const [page, setPage] = useState<PageId>(() => pageFromHash(window.location.hash))
  const [refreshMode, setRefreshMode] = useState<RefreshMode>('default')
  const [accountFilter, setAccountFilter] = useState('')
  const [selectedSelectors, setSelectedSelectors] = useState<string[]>([])
  const [importMode, setImportMode] = useState<ImportMode>('file')
  const [importPath, setImportPath] = useState('')
  const [importAlias, setImportAlias] = useState('')
  const [threshold5h, setThreshold5h] = useState('')
  const [thresholdWeekly, setThresholdWeekly] = useState('')
  const [rawCommand, setRawCommand] = useState('codex-auth status')
  const [commandLog, setCommandLog] = useState<CommandLogEntry[]>([])
  const [busyAction, setBusyAction] = useState<string | null>(null)

  const overviewQuery = useQuery({
    queryKey: ['overview', refreshMode],
    queryFn: () => getCliOverview(refreshMode),
    refetchOnWindowFocus: false,
    refetchInterval: overviewRefreshIntervalMs,
    refetchIntervalInBackground: false,
  })

  const accounts = overviewQuery.data?.accounts ?? []
  const activeAccount = accounts.find((account) => account.isActive) ?? null
  const validSelectorSet = new Set(accounts.map((account) => account.selector))
  const actionableSelectedSelectors = selectedSelectors.filter((selector) =>
    validSelectorSet.has(selector),
  )
  const deferredFilter = useDeferredValue(accountFilter)
  const filteredAccounts = accounts.filter((account) => {
    const query = deferredFilter.trim().toLowerCase()
    if (!query) {
      return true
    }

    return [account.label, account.email, account.selector, account.plan]
      .filter(Boolean)
      .some((value) => value?.toLowerCase().includes(query))
  })

  const allVisibleSelected =
    filteredAccounts.length > 0 &&
    filteredAccounts.every((account) => actionableSelectedSelectors.includes(account.selector))

  async function refreshOverview() {
    await queryClient.invalidateQueries({ queryKey: ['overview'] })
  }

  function appendLog(title: string, result: CliCommandResult) {
    startTransition(() => {
      setCommandLog((current) => [
        {
          id: Date.now(),
          title,
          result,
          createdAt: new Date().toISOString(),
        },
        ...current,
      ].slice(0, 10))
    })
  }

  async function performCommand(
    actionKey: string,
    title: string,
    operation: () => Promise<CliCommandResult>,
    options?: { refresh?: boolean; successMessage?: string },
  ) {
    setBusyAction(actionKey)
    const toastId = toast.loading(title)

    try {
      const result = await operation()
      appendLog(title, result)

      if (options?.refresh ?? true) {
        await refreshOverview()
      }

      toast.success(
        options?.successMessage ??
          preferredResultMessage(result, `${title} completed successfully.`),
        { id: toastId },
      )
    } catch (error) {
      const message = error instanceof Error ? error.message : `${title} failed.`
      toast.error(message, { id: toastId })
    } finally {
      setBusyAction(null)
    }
  }

  function toggleSelection(selector: string) {
    setSelectedSelectors((current) =>
      current.includes(selector)
        ? current.filter((item) => item !== selector)
        : [...current, selector],
    )
  }

  function toggleVisibleSelection() {
    const visibleSelectors = filteredAccounts.map((account) => account.selector)
    setSelectedSelectors((current) => {
      if (allVisibleSelected) {
        return current.filter((selector) => !visibleSelectors.includes(selector))
      }

      return Array.from(new Set([...current, ...visibleSelectors]))
    })
  }

  function normalizedThreshold(value: string) {
    const trimmed = value.trim()
    if (!trimmed) {
      return undefined
    }

    const parsed = Number.parseInt(trimmed, 10)
    return Number.isNaN(parsed) ? undefined : parsed
  }

  const currentUsage5h = activeAccount?.usage5h ?? 'n/a'
  const currentUsageWeekly = activeAccount?.usageWeekly ?? 'n/a'

  useEffect(() => {
    const nextHash = `#${page}`
    if (window.location.hash !== nextHash) {
      window.history.replaceState(null, '', nextHash)
    }
  }, [page])

  useEffect(() => {
    const onHashChange = () => {
      setPage(pageFromHash(window.location.hash))
    }

    window.addEventListener('hashchange', onHashChange)
    return () => window.removeEventListener('hashchange', onHashChange)
  }, [])

  return (
    <main className="min-h-screen bg-[var(--background)] p-2.5 text-[var(--foreground)]">
      <div className="flex min-h-[calc(100vh-1.25rem)] w-full flex-col">
        <header className="space-y-2 border-b border-[var(--border)] px-3 py-2.5">
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <p className="truncate text-sm font-semibold">
                {activeAccount?.label ?? 'No active account'}
              </p>
              <p className="truncate text-[11px] text-[var(--muted-foreground)]">
                {activeAccount?.email ?? overviewQuery.data?.cliPath ?? 'Waiting for codex-auth'}
              </p>
            </div>
            <Button
              size="sm"
              variant="outline"
              className="h-8 px-2"
              disabled={overviewQuery.isFetching || !!busyAction}
              onClick={() => void refreshOverview()}
            >
              <RefreshCw className={cn('size-4', overviewQuery.isFetching && 'animate-spin')} />
            </Button>
          </div>

          <div className="grid grid-cols-2 gap-1.5">
            <MetricTile label="5h" value={currentUsage5h} wrapValue />
            <MetricTile label="Weekly" value={currentUsageWeekly} wrapValue />
            <MetricTile
              label="Mode"
              value={overviewQuery.data?.status.usageMode ?? 'unknown'}
            />
            <MetricTile label="Accounts" value={String(accounts.length)} />
          </div>

          <div className="flex items-center gap-2">
            <Select
              className="h-8 min-w-0 flex-1 text-xs"
              value={refreshMode}
              onChange={(event) => setRefreshMode(event.target.value as RefreshMode)}
            >
              {refreshModeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </Select>
            <span className="truncate text-[10px] text-[var(--muted-foreground)]">
              Auto refresh {Math.round(overviewRefreshIntervalMs / 1000)}s · {busyAction ? busyAction : 'ready'}
            </span>
          </div>
        </header>

        <nav className="grid grid-cols-4 gap-1 border-b border-[var(--border)] px-2 py-2">
          {pages.map((item) => {
            const Icon = item.icon

            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setPage(item.id)}
                className={cn(
                  'flex flex-col items-center gap-1 rounded-lg px-2 py-1.5 text-[11px] font-medium transition',
                  page === item.id
                    ? 'bg-[var(--accent)] text-[var(--foreground)]'
                    : 'text-[var(--muted-foreground)] hover:bg-[var(--muted)]',
                )}
              >
                <Icon className="size-4" />
                <span>{item.label}</span>
              </button>
            )
          })}
        </nav>

        <section className="min-h-0 flex-1 px-3 py-2">
          {page === 'accounts' ? (
            <div className="flex h-full min-h-0 flex-col gap-2">
              <div className="flex items-center justify-between gap-2">
                <SectionTitle
                  title="Accounts"
                  help="Live output from codex-auth list. Filter, switch, and remove accounts without opening the terminal."
                />
                <div className="flex gap-1.5">
                  <Button size="sm" variant="outline" className="h-7 px-2" onClick={toggleVisibleSelection}>
                    {allVisibleSelected ? 'Clear' : 'Pick'}
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    className="h-7 px-2"
                    disabled={actionableSelectedSelectors.length === 0 || !!busyAction}
                    onClick={() =>
                      void performCommand(
                        'remove-selected',
                        'Removing selected accounts',
                        () => removeAccounts(actionableSelectedSelectors, { refreshMode }),
                        { successMessage: 'Selected accounts removed.' },
                      )
                    }
                  >
                    <Trash2 className="size-3.5" />
                  </Button>
                </div>
              </div>

              <Input
                className="h-8 text-xs"
                placeholder="Filter accounts"
                value={accountFilter}
                onChange={(event) => setAccountFilter(event.target.value)}
              />

              <div className="min-h-0 flex-1 space-y-1.5 overflow-y-auto pr-0.5">
                {overviewQuery.isLoading ? (
                  <CompactEmptyState text="Loading account inventory..." />
                ) : filteredAccounts.length === 0 ? (
                  <CompactEmptyState text="No accounts match the current filter." />
                ) : (
                  filteredAccounts.map((account) => (
                    <AccountItem
                      key={account.selector}
                      account={account}
                      isSelected={actionableSelectedSelectors.includes(account.selector)}
                      isWorking={busyAction === `switch:${account.selector}`}
                      onSelect={() => toggleSelection(account.selector)}
                      onSwitch={() =>
                        void performCommand(
                          `switch:${account.selector}`,
                          `Switching to ${account.label}`,
                          () => switchAccount(account.selector, refreshMode),
                          { successMessage: `Switched to ${account.label}.` },
                        )
                      }
                    />
                  ))
                )}
              </div>
            </div>
          ) : null}

          {page === 'add' ? (
            <div className="space-y-2">
              <SectionTitle
                title="Add Accounts"
                help="Start browser login, device auth login, or import auth data using the same codex-auth commands as the CLI."
              />

              <div className="grid grid-cols-2 gap-2">
                <Button
                  size="sm"
                  className="h-8"
                  disabled={!!busyAction}
                  onClick={() =>
                    void performCommand(
                      'login',
                      'Starting browser login',
                      () => startLogin(false),
                      { successMessage: 'Browser login flow finished.' },
                    )
                  }
                >
                  <LogIn className="size-4" />
                  Browser
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  className="h-8"
                  disabled={!!busyAction}
                  onClick={() =>
                    void performCommand(
                      'device-login',
                      'Starting device auth login',
                      () => startLogin(true),
                      { successMessage: 'Device auth login flow finished.' },
                    )
                  }
                >
                  <ShieldCheck className="size-4" />
                  Device
                </Button>
              </div>

              <Separator />

              <Select
                className="h-8 text-xs"
                value={importMode}
                onChange={(event) => setImportMode(event.target.value as ImportMode)}
              >
                {importModeOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </Select>
              <Input
                className="h-8 text-xs"
                placeholder={importMode === 'purge' ? 'Optional scan path' : 'File or folder path'}
                value={importPath}
                onChange={(event) => setImportPath(event.target.value)}
              />
              <Input
                className="h-8 text-xs"
                disabled={importMode === 'purge'}
                placeholder="Optional alias"
                value={importAlias}
                onChange={(event) => setImportAlias(event.target.value)}
              />
              <Button
                size="sm"
                className="h-8"
                disabled={!!busyAction}
                onClick={() =>
                  void performCommand(
                    'import',
                    'Running import',
                    () =>
                      runImport({
                        mode: importMode,
                        path: importPath.trim() || undefined,
                        alias: importMode === 'purge' ? undefined : importAlias.trim() || undefined,
                      }),
                    { successMessage: 'Import completed.' },
                  )
                }
              >
                <FolderInput className="size-4" />
                Run import
              </Button>
            </div>
          ) : null}

          {page === 'settings' ? (
            <div className="space-y-2">
              <SectionTitle
                title="Settings"
                help="Control auto-switch, update threshold values, change API mode, and run codex-auth clean."
              />

              <div className="rounded-lg border border-[var(--border)] px-3 py-2">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="text-sm font-medium">Auto-switch</p>
                    <p className="text-[11px] text-[var(--muted-foreground)]">
                      {overviewQuery.data?.status.autoSwitchLabel ?? 'Unknown'}
                    </p>
                  </div>
                  <Switch
                    checked={overviewQuery.data?.status.autoSwitchEnabled ?? false}
                    onCheckedChange={(checked) =>
                      void performCommand(
                        'toggle-auto-switch',
                        checked ? 'Enabling auto-switch' : 'Disabling auto-switch',
                        () => setAutoSwitch(checked),
                        { successMessage: `Auto-switch ${checked ? 'enabled' : 'disabled'}.` },
                      )
                    }
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <Input
                  className="h-8 text-xs"
                  inputMode="numeric"
                  placeholder="5h %"
                  value={threshold5h}
                  onChange={(event) => setThreshold5h(event.target.value)}
                />
                <Input
                  className="h-8 text-xs"
                  inputMode="numeric"
                  placeholder="Weekly %"
                  value={thresholdWeekly}
                  onChange={(event) => setThresholdWeekly(event.target.value)}
                />
              </div>

              <Button
                size="sm"
                variant="secondary"
                className="h-8"
                disabled={!!busyAction}
                onClick={() =>
                  void performCommand(
                    'set-thresholds',
                    'Updating auto-switch thresholds',
                    () =>
                      setAutoSwitchThresholds(
                        normalizedThreshold(threshold5h),
                        normalizedThreshold(thresholdWeekly),
                      ),
                    { successMessage: 'Thresholds updated.' },
                  )
                }
              >
                <Gauge className="size-4" />
                Save thresholds
              </Button>

              <Separator />

              <div className="grid grid-cols-2 gap-2">
                <Button
                  size="sm"
                  variant={overviewQuery.data?.status.usageMode === 'api' ? 'default' : 'outline'}
                  className="h-8"
                  disabled={!!busyAction}
                  onClick={() =>
                    void performCommand(
                      'api-enable',
                      'Enabling API mode',
                      () => setApiMode(true),
                      { successMessage: 'API mode enabled.' },
                    )
                  }
                >
                  API mode
                </Button>
                <Button
                  size="sm"
                  variant={overviewQuery.data?.status.usageMode !== 'api' ? 'default' : 'outline'}
                  className="h-8"
                  disabled={!!busyAction}
                  onClick={() =>
                    void performCommand(
                      'api-disable',
                      'Disabling API mode',
                      () => setApiMode(false),
                      { successMessage: 'Local-only mode enabled.' },
                    )
                  }
                >
                  Local mode
                </Button>
              </div>

              <div className="grid grid-cols-2 gap-2 text-xs">
                <InfoChip label="Usage" value={overviewQuery.data?.status.usageMode ?? 'unknown'} />
                <InfoChip label="Service" value={overviewQuery.data?.status.serviceStatus ?? 'unknown'} />
              </div>

              <Button
                size="sm"
                variant="ghost"
                className="h-8"
                disabled={!!busyAction}
                onClick={() =>
                  void performCommand(
                    'clean',
                    'Cleaning stale account files',
                    () => cleanAccounts(),
                    { successMessage: 'Cleanup completed.' },
                  )
                }
              >
                <BrushCleaning className="size-4" />
                Run clean
              </Button>
            </div>
          ) : null}

          {page === 'console' ? (
            <div className="flex h-full min-h-0 flex-col gap-2">
              <SectionTitle
                title="Console"
                help="Run exact codex-auth commands and keep a short local history of their output."
              />

              <Textarea
                className="min-h-20 text-xs"
                value={rawCommand}
                onChange={(event) => setRawCommand(event.target.value)}
                placeholder="codex-auth status"
              />
              <div className="flex gap-2">
                <Button
                  size="sm"
                  className="h-8"
                  disabled={!!busyAction || !rawCommand.trim()}
                  onClick={() =>
                    void performCommand(
                      'raw-command',
                      'Running raw command',
                      () => runRawCommand(rawCommand),
                      { refresh: true, successMessage: 'Raw command finished.' },
                    )
                  }
                >
                  <TerminalSquare className="size-4" />
                  Run
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  className="h-8"
                  disabled={!!busyAction}
                  onClick={() => setRawCommand('codex-auth --help')}
                >
                  Help
                </Button>
              </div>

              <div className="min-h-0 flex-1 space-y-1.5 overflow-y-auto rounded-lg border border-[var(--border)] p-2">
                {commandLog.length === 0 ? (
                  <CompactEmptyState text="Recent command output will appear here." />
                ) : (
                  commandLog.map((entry) => (
                    <div
                      key={entry.id}
                      className="rounded-lg border border-[var(--border)] bg-[var(--muted)]/40 p-2"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <p className="truncate text-xs font-medium">{entry.title}</p>
                        <span className="text-[10px] text-[var(--muted-foreground)]">
                          {new Date(entry.createdAt).toLocaleTimeString()}
                        </span>
                      </div>
                      <pre className="mt-1.5 overflow-x-auto whitespace-pre-wrap rounded-md bg-[var(--surface)] p-2 text-[10px] leading-5">
                        {formatResult(entry.result)}
                      </pre>
                    </div>
                  ))
                )}
              </div>
            </div>
          ) : null}
        </section>
      </div>
    </main>
  )
}

function SectionTitle({ title, help }: { title: string; help: string }) {
  return (
    <div className="flex items-center gap-2">
      <h2 className="text-sm font-semibold">{title}</h2>
      <HelpDot text={help} />
    </div>
  )
}

function HelpDot({ text }: { text: string }) {
  return (
    <div className="group relative">
      <div className="flex size-4 cursor-help items-center justify-center rounded-full border border-[var(--border)] text-[10px] font-semibold text-[var(--muted-foreground)]">
        ?
      </div>
      <div className="pointer-events-none absolute left-1/2 top-6 z-20 hidden w-48 -translate-x-1/2 rounded-lg border border-[var(--border)] bg-[var(--surface)] p-2 text-[11px] leading-5 shadow-lg group-hover:block group-focus-within:block">
        {text}
      </div>
    </div>
  )
}

function MetricTile({
  label,
  value,
  wrapValue = false,
}: {
  label: string
  value: string
  wrapValue?: boolean
}) {
  return (
    <div className="rounded-lg border border-[var(--border)] px-2 py-1.5">
      <span className="block text-[9px] font-semibold uppercase tracking-[0.16em] text-[var(--muted-foreground)]">
        {label}
      </span>
      <span
        className={cn(
          'mt-1 block text-xs font-medium',
          wrapValue ? 'break-words leading-4 whitespace-normal' : 'truncate',
        )}
      >
        {value}
      </span>
    </div>
  )
}

function InfoChip({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-[var(--border)] px-2.5 py-2">
      <span className="block text-[9px] font-semibold uppercase tracking-[0.16em] text-[var(--muted-foreground)]">
        {label}
      </span>
      <span className="mt-1 block truncate text-xs">{value}</span>
    </div>
  )
}

function CompactEmptyState({ text }: { text: string }) {
  return (
    <div className="rounded-lg border border-dashed border-[var(--border)] px-3 py-5 text-center text-xs text-[var(--muted-foreground)]">
      {text}
    </div>
  )
}

function AccountItem({
  account,
  isSelected,
  isWorking,
  onSelect,
  onSwitch,
}: {
  account: CliAccountOverview
  isSelected: boolean
  isWorking: boolean
  onSelect: () => void
  onSwitch: () => void
}) {
  return (
    <div
      className={cn(
        'rounded-lg border px-2.5 py-2',
        account.isActive ? 'border-[var(--ring)] bg-[var(--accent)]/45' : 'border-[var(--border)]',
      )}
    >
      <div className="flex items-start gap-2">
        <Checkbox checked={isSelected} onChange={() => onSelect()} />
        <div className="min-w-0 flex-1 space-y-1.5">
          <div className="flex items-start justify-between gap-2">
            <p className="truncate text-sm font-medium">{account.label}</p>
            <div className="flex shrink-0 gap-1">
              {account.isActive ? <Badge variant="success">Active</Badge> : null}
              <Badge variant="outline">#{account.index}</Badge>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-1.5 text-[10px] text-[var(--muted-foreground)]">
            <Badge variant="secondary">{account.plan}</Badge>
            <span>{account.usage5h}</span>
            <span>{account.usageWeekly}</span>
            <span>{account.lastActivity}</span>
          </div>

          <div className="flex justify-end">
            <Button
              size="sm"
              variant={account.isActive ? 'outline' : 'default'}
              className="h-7 px-2"
              disabled={account.isActive || isWorking}
              onClick={onSwitch}
            >
              {isWorking ? (
                <>
                  <RefreshCw className="size-3.5 animate-spin" />
                  Wait
                </>
              ) : account.isActive ? (
                'Current'
              ) : (
                'Switch'
              )}
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}

function preferredResultMessage(result: CliCommandResult, fallback: string) {
  const stdout = result.stdout.trim()
  if (stdout) {
    return stdout.split('\n').at(-1) ?? fallback
  }

  const stderr = result.stderr.trim()
  if (stderr) {
    return stderr.split('\n').at(-1) ?? fallback
  }

  return fallback
}

function formatResult(result: CliCommandResult) {
  const parts = [
    `Command: ${result.commandDisplay}`,
    `Exit code: ${String(result.exitCode ?? 'n/a')}`,
  ]

  if (result.stdout.trim()) {
    parts.push('', 'stdout:', result.stdout.trim())
  }

  if (result.stderr.trim()) {
    parts.push('', 'stderr:', result.stderr.trim())
  }

  return parts.join('\n')
}

export default App

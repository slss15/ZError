const SENSITIVE_KEY = /(api[-_]?key|authorization|token|password|secret|cookie)/i

const isTauriEnvironment = (): boolean => {
  if (typeof window === 'undefined') return false
  return (
    !!(window as any).__TAURI__ ||
    !!(window as any).__TAURI_INTERNALS__ ||
    window.location.protocol === 'tauri:'
  )
}

const sanitizeValue = (value: unknown, depth = 0): unknown => {
  if (value === null || value === undefined) return value
  if (depth > 5) return '[max-depth]'
  if (typeof value === 'string') {
    return value.length > 2000 ? `${value.slice(0, 2000)}…[truncated]` : value
  }
  if (typeof value === 'number' || typeof value === 'boolean') return value
  if (Array.isArray(value)) {
    return value.slice(0, 50).map(item => sanitizeValue(item, depth + 1))
  }
  if (typeof value === 'object') {
    const result: Record<string, unknown> = {}
    for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
      if (SENSITIVE_KEY.test(key)) {
        result[key] = '[redacted]'
      } else {
        result[key] = sanitizeValue(item, depth + 1)
      }
    }
    return result
  }
  return String(value)
}

export interface DiagnosticEventInput {
  level?: 'debug' | 'info' | 'warn' | 'error'
  scope: string
  event: string
  requestId?: string
  data?: Record<string, unknown>
}

/**
 * 写一条结构化诊断日志。
 * - 桌面端写入 %LOCALAPPDATA%\\ZError\\logs\\app-YYYYMMDD.jsonl
 * - 浏览器开发环境回退到 console.debug
 */
export const logDiagnostic = (input: DiagnosticEventInput): void => {
  const visibility =
    typeof document !== 'undefined' && document.visibilityState
      ? document.visibilityState
      : 'unknown'

  const payload = sanitizeValue({
    ...(input.data || {}),
    visibility
  }) as Record<string, unknown>

  if (!isTauriEnvironment()) {
    console.debug(`[${input.scope}] ${input.event}`, payload)
    return
  }

  import('@tauri-apps/api/core')
    .then(({ invoke }) =>
      invoke('append_diagnostic_log', {
        level: input.level || 'info',
        scope: input.scope,
        event: input.event,
        requestId: input.requestId || null,
        data: payload
      })
    )
    .catch(error => {
      console.warn('写入诊断日志失败:', error)
    })
}

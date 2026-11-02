import { convertFileSrc, isTauri } from '@tauri-apps/api/core'

const routes = ['remote', 'local', 'photo'] as const

/** 由 Tauri 按平台生成图片地址：Windows 使用 localhost，macOS 使用自定义协议。 */
export function protocolUrl(kind: (typeof routes)[number], target: string): string {
  return isTauri()
    ? convertFileSrc(`${kind}/${target}`, 'jmimg')
    : `http://jmimg.localhost/${kind}/${encodeURIComponent(target)}`
}

/** 后端返回的图片 URL 统一转成 WebView 可加载的地址。 */
export function toDisplayUrl(url: string, localPath?: string): string {
  if (localPath) return protocolUrl('local', localPath)
  if (!url) return ''
  // 章节接口返回的是尚未编码的 jmimg://photo/<章节>/<文件名>。
  for (const kind of routes) {
    const prefix = `jmimg://${kind}/`
    if (url.startsWith(prefix)) return protocolUrl(kind, url.slice(prefix.length))
  }
  for (const prefix of [
    'http://jmimg.localhost/',
    'https://jmimg.localhost/',
    'jmimg://localhost/'
  ]) {
    if (!url.startsWith(prefix)) continue
    try {
      const path = decodeURIComponent(url.slice(prefix.length))
      for (const kind of routes) {
        if (path.startsWith(`${kind}/`)) return protocolUrl(kind, path.slice(kind.length + 1))
      }
    } catch {
      return ''
    }
    return ''
  }
  return protocolUrl('remote', url)
}

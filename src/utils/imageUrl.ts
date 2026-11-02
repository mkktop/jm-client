/**
 * WebView2 会把自定义协议派发成 http://jmimg.localhost/...，
 * 这里统一使用这个形态，后端同时兼容 jmimg://。
 */
export function protocolUrl(kind: 'remote' | 'local' | 'photo', target: string): string {
  return `http://jmimg.localhost/${kind}/${encodeURIComponent(target)}`
}

/** 后端返回的图片 URL 统一转成 WebView 可加载的地址。 */
export function toDisplayUrl(url: string, localPath?: string): string {
  if (localPath) return protocolUrl('local', localPath)
  if (!url) return ''
  if (url.startsWith('jmimg://')) {
    return url.replace('jmimg://', 'http://jmimg.localhost/')
  }
  if (url.startsWith('http://jmimg.localhost/')) return url
  return protocolUrl('remote', url)
}

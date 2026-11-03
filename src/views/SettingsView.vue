<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../stores/app'
import { useComicStore, type BlockedTagScope } from '../stores/comic'
import AppIcon from '../components/AppIcon.vue'
import BrandMark from '../components/BrandMark.vue'
import AppUpdate from '../components/AppUpdate.vue'
import { useUpdateStore } from '../stores/updates'
const app = useAppStore()
const comic = useComicStore()
const updates = useUpdateStore()
const blockInput = ref('')
function addBlocked() {
  const value = blockInput.value.trim()
  if (!value) return
  comic.blockTag(value)
  blockInput.value = ''
}
function removeBlocked(tag: string) {
  comic.unblockTag(tag)
}
function toggleScope(scope: BlockedTagScope) {
  comic.setBlockedTagScope(scope, !comic.blockedTagScopes[scope])
}
const downloadDir = ref('')
const proxy = ref('')
const apiDomain = ref('')
const apiDomains = ref<string[]>([])
const imgHost = ref('')
const apiVersion = ref('')
const downloadFormat = ref('jpg')
const downloadConcurrency = ref(8)
const downloadInterval = ref(300)
const saving = ref('')
const messages = ref<Record<string, string>>({})
const cacheEnabled = ref(false)
const cacheLimit = ref(256)
const cacheUsed = ref(0)
const confirmingClear = ref(false)
function formatBytes(n: number): string {
  if (n < 1024) return n + ' B'
  if (n < 1024 * 1024) return (n / 1024).toFixed(1) + ' KB'
  if (n < 1024 ** 3) return (n / 1024 / 1024).toFixed(1) + ' MB'
  return (n / 1024 / 1024 / 1024).toFixed(2) + ' GB'
}
async function loadCacheStatus() {
  try {
    const status = await invoke<{ enabled: boolean; limit_mb: number; used_bytes: number }>(
      'get_image_cache_status'
    )
    cacheEnabled.value = status.enabled
    cacheLimit.value = status.limit_mb
    cacheUsed.value = status.used_bytes
  } catch {
    // 状态拉不到时保持默认展示
  }
}
async function persistCacheSettings(): Promise<boolean> {
  if (saving.value) return false
  saving.value = 'cache'
  messages.value.cache = ''
  try {
    await invoke('set_image_cache_settings', {
      enabled: cacheEnabled.value,
      limitMb: Math.max(16, Math.min(8192, Number(cacheLimit.value) || 256))
    })
    await loadCacheStatus()
    messages.value.cache = cacheEnabled.value ? '封面缓存已启用' : '封面缓存已关闭'
    return true
  } catch (e) {
    messages.value.cache = '保存失败：' + e
    return false
  } finally {
    saving.value = ''
  }
}
async function toggleCache() {
  const next = !cacheEnabled.value
  cacheEnabled.value = next
  if (!(await persistCacheSettings())) cacheEnabled.value = !next
}
async function saveCacheLimit() {
  await persistCacheSettings()
}
async function clearCache() {
  if (saving.value) return
  saving.value = 'cache'
  messages.value.cache = ''
  try {
    const freed = await invoke<number>('clear_image_cache')
    messages.value.cache = '已清空 ' + formatBytes(freed)
    await loadCacheStatus()
  } catch (e) {
    messages.value.cache = '清空失败：' + e
  } finally {
    saving.value = ''
  }
}
async function refreshDomains() {
  if (saving.value) return
  saving.value = 'domain'
  messages.value.domain = ''
  try {
    apiDomains.value = await invoke<string[]>('refresh_api_domains')
    messages.value.domain = '线路列表已刷新'
  } catch (e) {
    messages.value.domain = '刷新失败：' + e
  } finally {
    saving.value = ''
  }
}
onMounted(async () => {
  await app.loadDownloadDir()
  downloadDir.value = app.downloadDir
  const results = await Promise.allSettled([
    invoke<string | null>('get_proxy'),
    invoke<string>('get_api_domain'),
    invoke<string[]>('get_api_domains'),
    invoke<string>('get_img_host'),
    invoke<string>('get_api_version'),
    invoke<{ format: string; concurrency: number; interval_ms: number }>('get_download_settings'),
    loadCacheStatus()
  ])
  if (results[0].status === 'fulfilled') proxy.value = results[0].value || ''
  if (results[1].status === 'fulfilled') apiDomain.value = results[1].value
  if (results[2].status === 'fulfilled') apiDomains.value = results[2].value
  if (results[3].status === 'fulfilled') imgHost.value = results[3].value
  if (results[4].status === 'fulfilled') apiVersion.value = results[4].value
  if (results[5].status === 'fulfilled') {
    downloadFormat.value = results[5].value.format
    downloadConcurrency.value = results[5].value.concurrency
    downloadInterval.value = results[5].value.interval_ms
  }
})
async function pickDir() {
  if (saving.value) return
  try {
    const dir = await invoke<string | null>('pick_download_dir')
    if (!dir) return
    downloadDir.value = dir
    await save('directory')
  } catch (e) {
    messages.value.directory = '选择目录失败：' + e
  }
}
async function exportHistory() {
  if (saving.value) return
  saving.value = 'history'
  messages.value.history = ''
  try {
    const path = await invoke<string | null>('export_download_history')
    messages.value.history = path ? '已导出到 ' + path : '已取消导出'
  } catch (e) {
    messages.value.history = '导出失败：' + e
  } finally {
    saving.value = ''
  }
}
async function importHistory() {
  if (saving.value) return
  saving.value = 'history'
  messages.value.history = ''
  try {
    const result = await invoke<string | null>('import_download_history')
    messages.value.history = result || '已取消导入'
  } catch (e) {
    messages.value.history = '导入失败：' + e
  } finally {
    saving.value = ''
  }
}
async function clearAllData() {
  if (saving.value) return
  saving.value = 'clear'
  messages.value.clear = ''
  // 主题、布局与阅读进度等偏好保存在 WebView localStorage 中。
  try {
    localStorage.clear()
  } catch {
    /* 存储不可用时忽略 */
  }
  try {
    await invoke('clear_all_data')
    messages.value.clear = '已清除，正在重启…'
  } catch (e) {
    messages.value.clear = '清除失败：' + e
    saving.value = ''
    confirmingClear.value = false
  }
}
async function save(kind: string) {
  if (saving.value) return
  saving.value = kind
  messages.value[kind] = ''
  try {
    if (kind === 'directory') {
      if (!downloadDir.value.trim()) throw new Error('请填写下载目录')
      await invoke('set_download_dir', { dir: downloadDir.value.trim() })
      app.downloadDir = downloadDir.value.trim()
    }
    if (kind === 'proxy') {
      await invoke('set_proxy', { proxy: proxy.value.trim() })
      apiVersion.value = await invoke<string>('get_api_version')
      imgHost.value = await invoke<string>('get_img_host')
    }
    if (kind === 'domain') {
      if (!apiDomain.value.trim()) throw new Error('请选择 API 线路')
      await invoke('set_api_domain', { domain: apiDomain.value.trim() })
      await app.checkLogin()
    }
    if (kind === 'imghost') await invoke('set_img_host', { host: imgHost.value.trim() })
    if (kind === 'download') {
      await invoke('save_download_settings', {
        settings: {
          format: downloadFormat.value,
          concurrency: Math.max(1, Math.min(32, Number(downloadConcurrency.value) || 8)),
          interval_ms: Math.max(0, Math.min(5000, Number(downloadInterval.value) || 0))
        }
      })
    }
    messages.value[kind] =
      kind === 'proxy'
        ? proxy.value.trim()
          ? '代理已启用'
          : '已切换为直接连接'
        : kind === 'domain'
          ? app.isLoggedIn
            ? '线路已切换'
            : '线路已切换，需要重新登录'
          : '设置已保存'
  } catch (e) {
    messages.value[kind] = '保存失败：' + e
  } finally {
    saving.value = ''
  }
}
</script>
<template>
  <div class="page settings-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">PREFERENCES</div>
        <h1>偏好设置</h1>
        <p>调整界面外观、接口线路、下载设置和网络连接。</p>
      </div>
    </div>
    <div class="settings-layout">
      <div class="settings-form">
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="sun" />
            <div>
              <h2>外观与布局</h2>
              <p>选择最舒服的浏览方式</p>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>界面外观</strong>
              <p>阅读器始终使用独立的深色背景。</p>
            </div>
            <div class="segmented">
              <button
                :class="{ active: app.theme === 'light' }"
                @click="app.theme !== 'light' && app.toggleTheme()"
              >
                <AppIcon name="sun" :size="13" /> 浅色</button
              ><button
                :class="{ active: app.theme === 'dark' }"
                @click="app.theme !== 'dark' && app.toggleTheme()"
              >
                <AppIcon name="moon" :size="13" /> 深色
              </button>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>紧凑封面布局</strong>
              <p>在同一屏幕中展示更多漫画。</p>
            </div>
            <button
              class="switch"
              :class="{ on: app.compactCards }"
              role="switch"
              :aria-checked="app.compactCards"
              aria-label="紧凑封面布局"
              @click="app.toggleCompactCards"
            >
              <span />
            </button>
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="network" />
            <div>
              <h2>接口与图源</h2>
              <p>JM API 线路、图床与图片 CDN</p>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>API 线路</strong>
              <p>当前版本 {{ apiVersion || '未知' }}；切换线路后可能需要重新登录。</p>
            </div>
            <div class="dir-field">
              <select v-model="apiDomain" class="input cache-limit" aria-label="API 线路">
                <option v-for="domain in apiDomains" :key="domain" :value="domain">
                  {{ domain }}
                </option>
              </select>
              <button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="refreshDomains">
                刷新线路
              </button>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>图床 / 图源</strong>
              <p>留空使用 /setting 返回的自动图床；图片 CDN 也会自动 fallback。</p>
            </div>
            <input
              v-model="imgHost"
              class="input cache-limit"
              placeholder="例如 cdn-msp2.jmapiproxy2.cc"
            />
          </div>
          <div class="setting-actions">
            <button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="save('domain')">
              保存线路</button
            ><button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="save('imghost')">
              保存图床</button
            ><span
              role="status"
              class="setting-message"
              :class="{
                error: messages.domain?.includes('失败') || messages.imghost?.includes('失败')
              }"
              >{{ messages.domain || messages.imghost }}</span
            >
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="download" />
            <div>
              <h2>下载设置</h2>
              <p>图片格式、并发与限速</p>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>保存格式</strong>
              <p>JM 原图是打乱的 WebP，下载时会解码重排后重新编码。</p>
            </div>
            <div class="segmented">
              <button :class="{ active: downloadFormat === 'jpg' }" @click="downloadFormat = 'jpg'">
                JPEG
              </button>
              <button :class="{ active: downloadFormat === 'png' }" @click="downloadFormat = 'png'">
                PNG
              </button>
              <button
                :class="{ active: downloadFormat === 'webp' }"
                @click="downloadFormat = 'webp'"
              >
                WebP
              </button>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>下载并发</strong>
              <p>同时下载的图片数量，默认 8。</p>
            </div>
            <input
              v-model.number="downloadConcurrency"
              type="number"
              min="1"
              max="32"
              class="input cache-limit"
              aria-label="下载并发"
            />
          </div>
          <div class="setting-row">
            <div>
              <strong>每张间隔</strong>
              <p>两张图片之间的等待毫秒数，默认 300ms。</p>
            </div>
            <div class="cache-limit-field">
              <input
                v-model.number="downloadInterval"
                type="number"
                min="0"
                max="5000"
                step="50"
                class="input cache-limit"
                aria-label="下载间隔"
              /><span>ms</span>
            </div>
          </div>
          <div class="setting-actions">
            <button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="save('download')">
              保存下载设置</button
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.download?.includes('失败') }"
              >{{ messages.download }}</span
            >
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="image" />
            <div>
              <h2>封面缓存</h2>
              <p>封面与头像的本地磁盘缓存，默认关闭</p>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>启用封面缓存</strong>
              <p>关闭时每次从网络加载（仍走代理设置）。</p>
            </div>
            <button
              class="switch"
              :class="{ on: cacheEnabled }"
              role="switch"
              :aria-checked="cacheEnabled"
              aria-label="启用封面缓存"
              @click="toggleCache"
            >
              <span />
            </button>
          </div>
          <div class="setting-row">
            <div>
              <strong>容量上限</strong>
              <p>超出后按最久未使用自动清理，单位 MB。</p>
            </div>
            <div class="cache-limit-field">
              <input
                v-model.number="cacheLimit"
                type="number"
                min="16"
                max="8192"
                step="16"
                class="input cache-limit"
                aria-label="缓存容量上限（MB）"
                :disabled="!cacheEnabled"
              /><span>MB</span>
            </div>
          </div>
          <div class="setting-actions">
            <button
              class="btn btn-secondary btn-sm"
              :disabled="!!saving || !cacheEnabled"
              @click="saveCacheLimit"
            >
              保存上限</button
            ><button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="clearCache">
              清空缓存</button
            ><span role="status" class="setting-message cache-usage"
              >当前占用 {{ formatBytes(cacheUsed) }}</span
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.cache?.includes('失败') }"
              >{{ messages.cache }}</span
            >
          </div>
          <p class="hint">
            缓存仅包含封面与头像缩略图（单张超过 8MB 的异常响应不落盘），阅读内容不进入缓存。
          </p>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="folder" />
            <div>
              <h2>下载目录</h2>
              <p>设置漫画图片的保存位置</p>
            </div>
          </div>
          <div class="form-field">
            <span class="eyebrow">存储位置</span>
            <div class="dir-field">
              <input v-model="downloadDir" class="input" placeholder="输入本地文件夹路径" />
              <button
                class="btn btn-secondary dir-browse"
                :disabled="!!saving"
                title="打开系统目录选择框"
                @click="pickDir"
              >
                <AppIcon name="folder" :size="14" />浏览…
              </button>
            </div>
          </div>
          <p class="hint">新的下载任务会保存到此目录，已下载的文件保留在原位置。</p>
          <div class="setting-actions">
            <button
              class="btn btn-secondary btn-sm"
              :disabled="!!saving"
              @click="save('directory')"
            >
              {{ saving === 'directory' ? '保存中…' : '保存目录' }}</button
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.directory?.includes('失败') }"
              >{{ messages.directory }}</span
            >
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="download" />
            <div>
              <h2>下载记录备份</h2>
              <p>下载历史独立于本地文件，可导出保存，重装后导入恢复</p>
            </div>
          </div>
          <div class="setting-actions">
            <button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="exportHistory">
              {{ saving === 'history' ? '处理中…' : '导出记录' }}</button
            ><button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="importHistory">
              导入记录</button
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.history?.includes('失败') }"
              >{{ messages.history }}</span
            >
          </div>
          <p class="hint">
            导出为 JSON 文件；导入时与现有记录合并（章节取并集），不会覆盖或删除已有记录。
          </p>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="close" />
            <div>
              <h2>屏蔽标签</h2>
              <p>携带所选标签的漫画会从对应列表中隐藏</p>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>搜索页结果</strong>
              <p>关键词搜索与标签过滤的结果应用屏蔽。</p>
            </div>
            <button
              class="switch"
              :class="{ on: comic.blockedTagScopes.search }"
              role="switch"
              :aria-checked="comic.blockedTagScopes.search"
              aria-label="屏蔽标签作用于搜索页"
              @click="toggleScope('search')"
            >
              <span />
            </button>
          </div>
          <div class="setting-row">
            <div>
              <strong>发现页列表</strong>
              <p>排行榜与最新更新应用屏蔽。</p>
            </div>
            <button
              class="switch"
              :class="{ on: comic.blockedTagScopes.home }"
              role="switch"
              :aria-checked="comic.blockedTagScopes.home"
              aria-label="屏蔽标签作用于首页"
              @click="toggleScope('home')"
            >
              <span />
            </button>
          </div>
          <div class="setting-row">
            <div>
              <strong>分类浏览</strong>
              <p>分类与多分类合并的列表应用屏蔽。</p>
            </div>
            <button
              class="switch"
              :class="{ on: comic.blockedTagScopes.categories }"
              role="switch"
              :aria-checked="comic.blockedTagScopes.categories"
              aria-label="屏蔽标签作用于分类浏览"
              @click="toggleScope('categories')"
            >
              <span />
            </button>
          </div>
          <div class="setting-row">
            <div>
              <strong>每周必看</strong>
              <p>期刊推荐列表应用屏蔽。</p>
            </div>
            <button
              class="switch"
              :class="{ on: comic.blockedTagScopes.weekly }"
              role="switch"
              :aria-checked="comic.blockedTagScopes.weekly"
              aria-label="屏蔽标签作用于每周必看"
              @click="toggleScope('weekly')"
            >
              <span />
            </button>
          </div>
          <div class="form-field">
            <span class="eyebrow">标签列表</span>
            <div class="blocked-tags">
              <template v-if="comic.blockedTags.length"
                ><button
                  v-for="t in comic.blockedTags"
                  :key="t"
                  class="hot-tag blocked-chip"
                  :title="'点击取消屏蔽「' + t + '」'"
                  @click="removeBlocked(t)"
                >
                  {{ t }} ×
                </button></template
              ><span v-else class="setting-message">尚未屏蔽任何标签</span
              ><input
                v-model="blockInput"
                class="block-input"
                aria-label="添加屏蔽标签"
                placeholder="+ 添加屏蔽标签（回车确认）"
                @keydown.enter.prevent="addBlocked"
              />
            </div>
          </div>
          <p class="hint">
            标签需与漫画携带的标签精确一致；改动在对应列表下次加载时生效（重新搜索或重新进入页面）。
          </p>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="network" />
            <div>
              <h2>网络连接</h2>
              <p>配置漫画服务与图片下载的连接方式</p>
            </div>
          </div>
          <label class="form-field"
            ><span class="eyebrow">代理地址</span
            ><input
              v-model="proxy"
              class="input"
              placeholder="http://127.0.0.1:7890 或 socks5://127.0.0.1:7891"
          /></label>
          <p class="hint">支持 HTTP / SOCKS5 代理。留空后应用，即使用直接连接。</p>
          <div class="setting-actions">
            <button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="save('proxy')">
              {{ saving === 'proxy' ? '应用中…' : '应用连接设置' }}</button
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.proxy?.includes('失败') }"
              >{{ messages.proxy }}</span
            >
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="user" />
            <div>
              <h2>账号状态</h2>
              <p>你的收藏、评论、点赞和观看历史会跟随 JM 账号同步</p>
            </div>
          </div>
          <div class="setting-row">
            <span class="header-note"
              ><span class="status-dot" />{{
                app.isLoggedIn
                  ? '已登录 · ' + (comic.profile?.name || 'JM 账号')
                  : app.isGuest
                    ? '游客模式'
                    : '尚未登录'
              }}</span
            >
            <button v-if="app.isLoggedIn" class="btn btn-secondary btn-sm" @click="app.logout()">
              退出登录
            </button>
            <button v-else class="btn btn-primary btn-sm" @click="app.logout()">去登录</button>
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="alert" />
            <div>
              <h2>清除数据</h2>
              <p>删除本机保存的全部数据，恢复为全新安装状态</p>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>清除所有本地数据</strong>
              <p>
                删除登录状态、全部设置、本地下载记录、封面缓存与更新缓存。下载目录中已保存的漫画文件不会被删除。
              </p>
            </div>
            <div v-if="confirmingClear" class="clear-actions">
              <button class="btn btn-danger btn-sm" :disabled="!!saving" @click="clearAllData">
                {{ saving === 'clear' ? '清除中…' : '确认清除' }}
              </button>
              <button
                class="btn btn-secondary btn-sm"
                :disabled="!!saving"
                @click="confirmingClear = false"
              >
                取消
              </button>
            </div>
            <button
              v-else
              class="btn btn-danger btn-sm"
              :disabled="!!saving"
              @click="confirmingClear = true"
            >
              清除所有数据
            </button>
          </div>
          <p class="hint">
            清除后应用会自动重启，此操作不可撤销；如需保留下载记录，请先在上方「下载记录备份」中导出。
          </p>
          <span
            role="status"
            class="setting-message"
            :class="{ error: messages.clear?.includes('失败') }"
            >{{ messages.clear }}</span
          >
        </section>
        <AppUpdate />
      </div>
      <aside class="settings-aside">
        <BrandMark />
        <h3>JM DESKTOP</h3>
        <p>
          专注看本的桌面客户端。支持发现、分类、排行榜、每周必看、账号收藏、章节下载和阅读进度记忆。
        </p>
        <small
          >JM DESKTOP<template v-if="updates.currentVersion">
            · v{{ updates.currentVersion }}</template
          ></small
        >
      </aside>
    </div>
  </div>
</template>
<style scoped>
.clear-actions {
  display: flex;
  gap: 10px;
  flex-shrink: 0;
}
</style>

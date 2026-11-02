<script setup lang="ts">
import { onActivated, onDeactivated, ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'
import AppIcon from '../components/AppIcon.vue'
import EmptyState from '../components/EmptyState.vue'
import Paginator from '../components/Paginator.vue'
interface DownloadTask {
  id: string
  comic_id: string
  comic_title: string
  chapter_id: string
  episode_order: number
  episode_title: string
  status: string
  total_images: number
  downloaded_images: number
  save_path: string
}
interface HistoryEntry {
  comic_id: string
  title: string
  chapter_ids: string[]
  first_downloaded_at?: string | null
  last_downloaded_at?: string | null
}
const router = useRouter()

defineOptions({ name: 'DownloadsView' })
const tasks = ref<DownloadTask[]>([])
const history = ref<HistoryEntry[]>([])
const historyLoading = ref(true)
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const filter = ref('all')
const activeTasks = computed(() =>
  tasks.value.filter((t) => ['downloading', 'pending', 'paused'].includes(t.status))
)
const completedTasks = computed(() => tasks.value.filter((t) => t.status === 'completed'))
const failedTasks = computed(() =>
  tasks.value.filter((t) => ['failed', 'cancelled'].includes(t.status))
)
const filtered = computed(() =>
  filter.value === 'active'
    ? activeTasks.value
    : filter.value === 'completed'
      ? completedTasks.value
      : filter.value === 'failed'
        ? failedTasks.value
        : tasks.value
)
async function loadTasks() {
  if (refreshing.value) return
  refreshing.value = true
  try {
    tasks.value = await invoke<DownloadTask[]>('get_download_queue')
    error.value = ''
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
    refreshing.value = false
  }
}
async function loadHistory() {
  try {
    history.value = await invoke<HistoryEntry[]>('get_download_history')
  } catch {
    history.value = []
  } finally {
    historyLoading.value = false
  }
}
const finishedCount = computed(() => completedTasks.value.length + failedTasks.value.length)
const clearing = ref(false)
async function clearFinished() {
  if (clearing.value || !finishedCount.value) return
  clearing.value = true
  try {
    await invoke('clear_finished_download_tasks')
    await loadTasks()
  } catch {
    // 清理失败时保持列表原样，下次刷新重试
  } finally {
    clearing.value = false
  }
}
const historyDate = (iso?: string | null) => (iso ? iso.slice(0, 10) : '')
// 历史可能积累到几万条，前端分页渲染，避免一次性挂满 DOM。
const HISTORY_PAGE_SIZE = 60
const historyPage = ref(1)
const historyPages = computed(() => Math.max(1, Math.ceil(history.value.length / HISTORY_PAGE_SIZE)))
const pagedHistory = computed(() =>
  history.value.slice(
    (historyPage.value - 1) * HISTORY_PAGE_SIZE,
    historyPage.value * HISTORY_PAGE_SIZE
  )
)
watch(history, () => {
  historyPage.value = 1
})
const statusText = (s: string) =>
  ({
    pending: '等待中',
    downloading: '下载中',
    paused: '已暂停',
    completed: '已完成',
    failed: '下载失败',
    cancelled: '已取消'
  })[s] || s
const progress = (t: DownloadTask) =>
  t.total_images
    ? Math.max(0, Math.min(100, Math.round((t.downloaded_images / t.total_images) * 100)))
    : 0
const acting = ref(new Set<string>())
async function control(task: DownloadTask, action: 'pause' | 'resume' | 'cancel') {
  if (acting.value.has(task.id)) return
  const next = new Set(acting.value)
  next.add(task.id)
  acting.value = next
  try {
    await invoke(`${action}_download`, { taskId: task.id })
    await loadTasks()
  } catch (e) {
    error.value = String(e)
  } finally {
    const done = new Set(acting.value)
    done.delete(task.id)
    acting.value = done
  }
}
let timer: ReturnType<typeof setInterval> | undefined
// 页面被 keep-alive 缓存：激活时轮询，切走时停止，避免后台空转。
onActivated(() => {
  loadTasks()
  loadHistory()
  timer = setInterval(loadTasks, 3000)
})
onDeactivated(() => clearInterval(timer))
</script>
<template>
  <div class="page downloads-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">DOWNLOAD QUEUE</div>
        <h1>下载管理</h1>
        <p>查看下载队列、章节进度与完成状态。</p>
      </div>
      <span class="header-note"><span class="status-dot" />每 3 秒自动更新</span>
    </div>
    <div class="stats-bar">
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="download" /></span>
        <div>
          <strong class="num">{{ activeTasks.length }}</strong
          ><span class="label">进行中 / 等待中</span>
        </div>
      </div>
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="check" /></span>
        <div>
          <strong class="num">{{ completedTasks.length }}</strong
          ><span class="label">已完成任务</span>
        </div>
      </div>
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="alert" /></span>
        <div>
          <strong class="num">{{ failedTasks.length }}</strong
          ><span class="label">未完成任务</span>
        </div>
      </div>
    </div>
    <div class="section-heading">
      <div class="segmented">
        <button
          v-for="tab in [
            { value: 'all', label: '全部任务' },
            { value: 'active', label: '进行中' },
            { value: 'completed', label: '已完成' },
            { value: 'failed', label: '失败' }
          ]"
          :key="tab.value"
          :class="{ active: filter === tab.value }"
          @click="filter = tab.value"
        >
          {{ tab.label }}
        </button>
      </div>
      <div class="section-controls">
        <button
          class="text-btn"
          :disabled="clearing || !finishedCount"
          title="清掉已完成/失败的任务记录；本地文件、书库与下载历史不受影响"
          @click="clearFinished"
        >
          <AppIcon name="close" :size="14" />{{
            clearing ? '清理中…' : '清理已结束任务'
          }}</button
        ><router-link to="/library" class="text-btn"
          >打开本地书库 <AppIcon name="arrow" :size="14"
        /></router-link>
      </div>
    </div>
    <div v-if="loading" class="loading"><span class="spinner" />正在读取下载队列…</div>
    <template v-else
      ><div v-if="error && tasks.length" class="notice error" role="alert">
        {{ error }}<button class="text-btn" @click="loadTasks">刷新</button>
      </div>
      <EmptyState
        v-if="error && !tasks.length"
        error
        title="暂时无法读取下载任务"
        :description="error"
        ><button class="btn btn-primary" @click="loadTasks">重试</button></EmptyState
      ><EmptyState
        v-else-if="!filtered.length"
        icon="download"
        :title="tasks.length ? '这个分类下还没有任务' : '暂无下载任务'"
        :description="
          tasks.length
            ? '切换分类查看其他下载任务。'
            : '在漫画详情的章节列表中选择下载，任务会显示在这里。'
        "
        ><router-link v-if="!tasks.length" to="/" class="btn btn-primary"
          >去发现漫画 <AppIcon name="arrow" :size="15" /></router-link
      ></EmptyState>
      <div v-else class="task-list">
        <article v-for="task in filtered" :key="task.id" class="task-card">
          <span class="task-icon"
            ><AppIcon :name="task.status === 'completed' ? 'check' : 'download'"
          /></span>
          <div class="task-info">
            <div class="task-title">{{ task.comic_title }}</div>
            <p class="task-episode">{{ task.episode_title }}</p>
            <div class="task-path" :title="task.save_path">{{ task.save_path }}</div>
          </div>
          <div class="task-progress">
            <div
              class="progress-bar"
              role="progressbar"
              :aria-label="task.episode_title + ' 下载进度'"
              :aria-valuenow="progress(task)"
              :aria-valuemin="0"
              :aria-valuemax="100"
            >
              <div
                class="progress-fill"
                :style="{
                  width: progress(task) + '%',
                  background:
                    task.status === 'completed'
                      ? 'var(--success)'
                      : task.status === 'failed'
                        ? 'var(--error)'
                        : undefined
                }"
              />
            </div>
            <div class="progress-text">
              <span>{{ task.downloaded_images }} / {{ task.total_images }} 张</span
              ><span>{{ progress(task) }}%</span>
            </div>
          </div>
          <span class="task-status" :class="task.status">{{ statusText(task.status) }}</span>
          <div class="task-actions">
            <button
              v-if="['downloading', 'pending'].includes(task.status)"
              class="icon-btn"
              title="暂停"
              :disabled="acting.has(task.id)"
              @click="control(task, 'pause')"
            >
              <AppIcon name="window-minimize" :size="15" />
            </button>
            <button
              v-if="task.status === 'paused'"
              class="icon-btn"
              title="恢复"
              :disabled="acting.has(task.id)"
              @click="control(task, 'resume')"
            >
              <AppIcon name="refresh" :size="15" />
            </button>
            <button
              v-if="['downloading', 'pending', 'paused'].includes(task.status)"
              class="icon-btn"
              title="取消"
              :disabled="acting.has(task.id)"
              @click="control(task, 'cancel')"
            >
              <AppIcon name="close" :size="15" />
            </button>
          </div>
        </article>
      </div></template
    >
    <section class="detail-section downloads-history">
      <div class="section-heading">
        <h2>
          下载历史<small v-if="!historyLoading"
            >{{ history.length }} 部 · 本地文件删除后记录仍保留</small
          >
        </h2>
      </div>
      <div v-if="historyLoading" class="loading">
        <span class="spinner" />正在读取下载历史…
      </div>
      <p v-else-if="!history.length" class="hint">
        还没有下载历史。整章下载完成后会在这里留下记录，可在设置中导出备份。
      </p>
      <div v-else class="history-grid">
        <button
          v-for="entry in pagedHistory"
          :key="entry.comic_id"
          class="history-card"
          :title="'查看《' + entry.title + '》'"
          @click="router.push('/comic/' + entry.comic_id)"
        >
          <AppIcon name="book" :size="16" />
          <span class="history-info">
            <strong>{{ entry.title }}</strong>
            <small
              >{{ entry.chapter_ids.length }} 话 · 最后下载
              {{ historyDate(entry.last_downloaded_at) }}</small
            >
          </span>
          <AppIcon name="chevron-right" :size="14" />
        </button>
      </div>
      <Paginator
        v-if="!historyLoading && historyPages > 1"
        :page="historyPage"
        :pages="historyPages"
        @change="historyPage = $event"
      />
    </section>
  </div>
</template>
<style scoped>
.task-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}
</style>

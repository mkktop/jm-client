<script setup lang="ts">
import { onMounted, ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'
import AppIcon from '../components/AppIcon.vue'
import EmptyState from '../components/EmptyState.vue'
import Paginator from '../components/Paginator.vue'
import { protocolUrl } from '../utils/imageUrl'
interface LocalComic {
  comic_id: string
  title: string
  author?: string
  finished?: boolean
  pages_count?: number
  eps_count?: number
  downloaded_episodes?: string[]
  cover_path?: string
}
const PAGE_SIZE = 60
const router = useRouter()

defineOptions({ name: 'LibraryView' })
const comics = ref<LocalComic[]>([])
const stats = ref({
  total_comics: 0,
  total_episodes: 0,
  total_images: 0,
  total_size_bytes: 0
})
function formatBytes(n: number): string {
  if (n < 1024) return n + ' B'
  if (n < 1024 * 1024) return (n / 1024).toFixed(1) + ' KB'
  if (n < 1024 ** 3) return (n / 1024 / 1024).toFixed(1) + ' MB'
  return (n / 1024 / 1024 / 1024).toFixed(2) + ' GB'
}
const loading = ref(false)
const error = ref('')
const query = ref('')
const page = ref(1)
const failedCovers = ref(new Set<string>())
const resultsTop = ref<HTMLElement>()
const filtered = computed(() =>
  comics.value.filter((c) =>
    (c.title + ' ' + (c.author || '')).toLowerCase().includes(query.value.toLowerCase())
  )
)
const pages = computed(() => Math.max(1, Math.ceil(filtered.value.length / PAGE_SIZE)))
const paged = computed(() =>
  filtered.value.slice((page.value - 1) * PAGE_SIZE, page.value * PAGE_SIZE)
)
watch(query, () => {
  page.value = 1
})
async function loadLibrary() {
  if (loading.value) return
  loading.value = true
  error.value = ''
  try {
    // 先核对磁盘清掉本地已删除的内容（下载历史保留），再读列表。
    await invoke<number>('cleanup_library')
    const [items, counts] = await Promise.all([
      invoke<LocalComic[]>('get_library', { limit: 100000, offset: 0 }),
      invoke<typeof stats.value>('get_library_stats')
    ])
    comics.value = items
    stats.value = counts
    page.value = 1
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}
function goToPage(target: number) {
  page.value = target
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
}
onMounted(() => loadLibrary())
</script>
<template>
  <div class="page library-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">LOCAL COMICS LIBRARY</div>
        <h1>本地书库</h1>
        <p>查看已下载到本地的漫画和章节。</p>
      </div>
      <button class="btn btn-secondary btn-sm" :disabled="loading" @click="loadLibrary()">
        <AppIcon name="refresh" :size="14" />刷新书库
      </button>
    </div>
    <div class="stats-bar">
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="book" /></span>
        <div>
          <strong class="num">{{ stats.total_comics }}</strong
          ><span class="label">收藏于本地的漫画</span>
        </div>
      </div>
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="layers" /></span>
        <div>
          <strong class="num">{{ stats.total_episodes }}</strong
          ><span class="label">已下载章节</span>
        </div>
      </div>
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="image" /></span>
        <div>
          <strong class="num">{{ stats.total_images.toLocaleString() }}</strong
          ><span class="label">已保存图片</span>
        </div>
      </div>
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="folder" /></span>
        <div>
          <strong class="num">{{ formatBytes(stats.total_size_bytes || 0) }}</strong
          ><span class="label">磁盘占用</span>
        </div>
      </div>
    </div>
    <div v-if="comics.length" ref="resultsTop" class="library-toolbar">
      <input
        v-model="query"
        class="input"
        placeholder="在本地书库中搜索…"
        aria-label="搜索本地书库"
      /><span class="header-note"
        >{{ filtered.length }} 部作品<span v-if="pages > 1"> · {{ pages }} 页</span></span
      >
    </div>
    <div v-if="loading && !comics.length" class="loading">
      <span class="spinner" />正在整理你的书架…
    </div>
    <EmptyState v-else-if="error" error title="书库暂时没有打开" :description="error"
      ><button class="btn btn-primary" @click="loadLibrary()">重新打开</button></EmptyState
    ><EmptyState
      v-else-if="!comics.length"
      title="本地书库为空"
      description="在漫画的章节列表中点击下载，完成后就能在这里找到它。"
      icon="library"
      ><router-link class="btn btn-primary" to="/"
        >发现漫画 <AppIcon name="arrow" :size="15" /></router-link></EmptyState
    ><EmptyState
      v-else-if="!filtered.length"
      title="书架上还没有这部作品"
      description="试试其他关键词。"
      icon="search"
    />
    <div v-else class="library-grid">
      <button
        v-for="comic in paged"
        :key="comic.comic_id"
        class="local-comic"
        @click="router.push('/comic/' + comic.comic_id)"
      >
        <span class="local-cover"
          ><img
            v-if="comic.cover_path && !failedCovers.has(comic.comic_id)"
            :src="protocolUrl('remote', comic.cover_path || '')"
            :alt="comic.title"
            loading="lazy"
            @error="failedCovers.add(comic.comic_id)" /><AppIcon
            v-else
            name="book"
            :size="27" /></span
        ><span class="local-info"
          ><strong>{{ comic.title }}</strong
          ><span>{{ comic.author || '作者未署名' }}</span
          ><span>{{ comic.eps_count || 0 }} 章节 · {{ comic.pages_count || 0 }} 页</span
          ><small>已下载 {{ comic.downloaded_episodes?.length || 0 }} 章</small></span
        >
      </button>
    </div>
    <Paginator
      v-if="!error && filtered.length"
      :page="page"
      :pages="pages"
      :loading="loading"
      @change="goToPage"
    />
  </div>
</template>

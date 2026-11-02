<script setup lang="ts">
import { onMounted, computed, ref, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from './stores/app'
import { useComicStore } from './stores/comic'
import Sidebar from './components/Sidebar.vue'
import AppIcon from './components/AppIcon.vue'
import LoginView from './views/LoginView.vue'
import WindowControls from './components/WindowControls.vue'
import BrandMark from './components/BrandMark.vue'
import { desktopWindow, dragWindow, syncWindowTheme } from './utils/desktopWindow'
const appStore = useAppStore()
const comicStore = useComicStore()
const route = useRoute()
const router = useRouter()
const ready = ref(false)
const query = ref('')
const main = ref<HTMLElement>()
const isReader = computed(() => route.name === 'reader')
const nativeWindow = !!desktopWindow()
watch(
  [() => appStore.theme, isReader],
  ([theme, reader]) => syncWindowTheme(reader ? 'dark' : theme),
  { immediate: true }
)
const pageName = computed(
  () =>
    ({
      home: '发现',
      categories: '分类浏览',
      ranking: '排行榜',
      weekly: '每周必看',
      search: '搜索',
      favorites: '我的收藏',
      history: '观看历史',
      library: '本地书库',
      downloads: '下载管理',
      settings: '偏好设置',
      'comic-detail': '漫画详情'
    })[String(route.name)] || '发现'
)
// 列表页用 keep-alive 缓存；离开时记录滚动位置，返回时恢复而不是重置。
const LIST_VIEW_NAMES = new Set([
  'home',
  'categories',
  'ranking',
  'weekly',
  'search',
  'favorites',
  'history',
  'library',
  'downloads'
])
const scrollPositions = new Map<string, number>()
watch(
  () => route.fullPath,
  async (_, from) => {
    const fromName = String(router.resolve(from).name || '')
    if (fromName && LIST_VIEW_NAMES.has(fromName)) {
      scrollPositions.set(from, main.value?.scrollTop ?? 0)
    }
    comicStore.cancelRequests()
    comicStore.fetchDownloadStates()
    query.value = String(route.query.q || '')
    await nextTick()
    const isListView = LIST_VIEW_NAMES.has(String(route.name || ''))
    const restore = isListView ? scrollPositions.get(route.fullPath) : undefined
    main.value?.scrollTo({ top: restore ?? 0, behavior: 'instant' })
  }
)
function search() {
  if (query.value.trim()) router.push({ path: '/search', query: { q: query.value.trim() } })
}
onMounted(async () => {
  await appStore.checkLogin()
  comicStore.fetchDownloadStates()
  ready.value = true
  if (appStore.canEnter) await appStore.loadDownloadDir()
})
</script>
<template>
  <div
    class="app-container"
    :data-theme="appStore.theme"
    :class="{ 'compact-cards': appStore.compactCards }"
  >
    <header
      v-if="nativeWindow && (!ready || !appStore.canEnter)"
      class="standalone-windowbar"
      @mousedown="dragWindow"
    >
      <WindowControls />
    </header>
    <div v-if="!ready" class="app-start">
      <BrandMark /><span class="spinner" />
      <p>正在打开 JM 客户端</p>
    </div>
    <LoginView v-else-if="!appStore.canEnter" />
    <template v-else>
      <Sidebar v-if="!isReader" />
      <div
        class="workspace"
        :class="{ collapsed: appStore.sidebarCollapsed, fullscreen: isReader }"
      >
        <header v-if="!isReader" class="topbar" @mousedown="dragWindow">
          <div class="breadcrumb">
            <span>JM Client</span><AppIcon name="chevron-right" :size="13" /><strong>{{
              pageName
            }}</strong>
          </div>
          <form class="global-search" @submit.prevent="search">
            <AppIcon name="search" :size="17" /><input
              v-model="query"
              aria-label="全局搜索"
              placeholder="搜索漫画、作者、标签或角色"
            /><kbd>↵</kbd>
          </form>
          <div class="topbar-actions">
            <button
              class="icon-btn"
              :title="appStore.theme === 'light' ? '切换深色外观' : '切换浅色外观'"
              @click="appStore.toggleTheme"
            >
              <AppIcon :name="appStore.theme === 'light' ? 'moon' : 'sun'" />
            </button>
          </div>
          <WindowControls />
        </header>
        <main ref="main" class="main-content" :class="{ fullscreen: isReader }">
          <router-view v-slot="{ Component }"
            ><transition name="fade" mode="out-in"
              ><keep-alive
                :include="[
                  'HomeView',
                  'CategoriesView',
                  'RankingView',
                  'WeeklyView',
                  'SearchView',
                  'FavoritesView',
                  'HistoryView',
                  'LibraryView',
                  'DownloadsView'
                ]"
                :max="12"
                ><component :is="Component" :key="route.path" /></keep-alive
              ></transition
          ></router-view>
        </main>
      </div>
    </template>
  </div>
</template>

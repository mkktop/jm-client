<script setup lang="ts">
import { onMounted, computed, ref, onUnmounted, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import AppIcon from '../components/AppIcon.vue'
import EmptyState from '../components/EmptyState.vue'
import { readPreference, writePreference, readingProgress } from '../utils/readerPreferences'
import WindowControls from '../components/WindowControls.vue'
import { dragWindow } from '../utils/desktopWindow'

const route = useRoute()
const router = useRouter()
const store = useComicStore()
const comicId = String(route.params.comicId)
const chapterId = String(route.params.chapterId)
const saved = readingProgress(comicId)
const currentIndex = ref(0)
const savedMode = readPreference<string>('jm-reader-mode', 'scroll')
const mode = ref<'scroll' | 'page'>(savedMode === 'page' ? 'page' : 'scroll')
const width = ref(String(readPreference<string>('jm-reader-width', '800')))
const zen = ref(false)
const scrollContainer = ref<HTMLElement>()
const pictures = computed(() => store.pictures)
const currentPic = computed(() => pictures.value[currentIndex.value])
const total = computed(() => Math.max(store.pictureTotal, pictures.value.length))
const pendingRestore = ref(-1)
const failed = ref(new Set<string>())
const preloaded = new Set<string>()
const preloadPool: HTMLImageElement[] = []
const title = computed(() =>
  store.currentComic?._id === comicId ? store.currentComic.title : '沉浸阅读'
)
const epOrder = computed(
  () => store.episodes.find((episode) => episode._id === chapterId)?.order ?? 0
)
const episodeIndex = computed(() =>
  store.episodes.findIndex((episode) => episode._id === chapterId)
)
const nextEpisode = computed(() =>
  episodeIndex.value >= 0 ? store.episodes[episodeIndex.value + 1] : undefined
)
let observer: IntersectionObserver | undefined
let active = true
let restoring = false
function persist() {
  if (pictures.value.length)
    writePreference('jm-progress-' + comicId, {
      chapterId,
      page: currentIndex.value
    })
}
watch(currentIndex, persist)
watch(mode, async (next) => {
  writePreference('jm-reader-mode', next)
  await nextTick()
  if (next === 'scroll') {
    observePages()
    goTo(currentIndex.value)
  }
})
watch(width, (value) => writePreference('jm-reader-width', value))
async function load() {
  await store.fetchPictures(comicId, chapterId)
  if (!active) return
  const target = saved?.chapterId === chapterId ? saved.page : 0
  currentIndex.value = Math.max(0, Math.min(target, pictures.value.length - 1))
  if (target > currentIndex.value) pendingRestore.value = target
  await nextTick()
  if (mode.value === 'scroll') {
    observePages()
    goTo(currentIndex.value)
  }
  preloadAhead()
}
function observePages() {
  observer?.disconnect()
  if (!scrollContainer.value) return
  observer = new IntersectionObserver(
    (entries) => {
      if (restoring) return
      const visible = entries
        .filter((entry) => entry.isIntersecting)
        .sort((a, b) => b.intersectionRatio - a.intersectionRatio)
      if (visible[0]) {
        currentIndex.value = Number((visible[0].target as HTMLElement).dataset.index)
      }
    },
    { root: scrollContainer.value, threshold: [0, 0.1, 0.25, 0.5, 0.75] }
  )
  scrollContainer.value.querySelectorAll('[data-index]').forEach((el) => observer?.observe(el))
}
function goTo(index: number) {
  currentIndex.value = Math.max(0, Math.min(index, pictures.value.length - 1))
  if (mode.value === 'scroll' && scrollContainer.value) {
    const el = scrollContainer.value.querySelector<HTMLElement>(
      '[data-index="' + currentIndex.value + '"]'
    )
    if (el) {
      restoring = true
      scrollContainer.value.scrollTop = el.offsetTop - scrollContainer.value.offsetTop
      requestAnimationFrame(() => {
        restoring = false
      })
    }
  }
}
function handleKey(event: KeyboardEvent) {
  if ((event.target as HTMLElement)?.matches('input, select, textarea')) return
  if (event.key === 'Escape') {
    if (zen.value) zen.value = false
    else back()
  }
  if (event.key.toLowerCase() === 'f') zen.value = !zen.value
  if (
    mode.value === 'page' &&
    ['ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp', ' '].includes(event.key)
  ) {
    event.preventDefault()
    goTo(currentIndex.value + (['ArrowLeft', 'ArrowUp'].includes(event.key) ? -1 : 1))
  }
}
function back() {
  const previous = router.options.history.state.back
  if (typeof previous === 'string') {
    const target = router.resolve(previous)
    if (target.name === 'comic-detail' && target.params.id === comicId) {
      router.back()
      return
    }
  }
  router.replace({ name: 'comic-detail', params: { id: comicId } })
}
function retry(id: string) {
  failed.value.delete(id)
}
function preloadAhead() {
  for (let i = currentIndex.value + 1; i <= currentIndex.value + 2; i++) {
    const pic = pictures.value[i]
    if (!pic) continue
    const url = store.getImageUrl(pic)
    if (url && !preloaded.has(url)) {
      preloaded.add(url)
      const img = new Image()
      img.src = url
      preloadPool.push(img)
    }
  }
}
watch(currentIndex, preloadAhead)
watch(
  () => pictures.value.length,
  async () => {
    if (!active) return
    if (pendingRestore.value >= 0 && pictures.value.length > pendingRestore.value) {
      const target = pendingRestore.value
      pendingRestore.value = -1
      await nextTick()
      goTo(target)
    }
    await nextTick()
    if (mode.value === 'scroll') observePages()
    preloadAhead()
  }
)
onMounted(async () => {
  if (!store.episodes.length) await store.fetchComic(comicId)
  await load()
  window.addEventListener('keydown', handleKey)
})
onUnmounted(() => {
  active = false
  observer?.disconnect()
  window.removeEventListener('keydown', handleKey)
  persist()
})
</script>
<template>
  <div class="reader-view" :class="{ zen }">
    <header class="reader-header" @mousedown="dragWindow">
      <button class="text-btn" @click="back"><AppIcon name="back" :size="17" />返回作品</button>
      <div class="reader-heading">
        <h1>{{ title }}</h1>
        <p>第 {{ epOrder }} 话 · {{ store.picturesLocal ? '离线阅读' : '在线阅读' }}</p>
      </div>
      <div class="reader-controls">
        <select v-model="width" class="input reader-width" aria-label="阅读宽度">
          <option value="640">窄幅</option>
          <option value="800">适中</option>
          <option value="1100">宽幅</option>
        </select>
        <div class="segmented">
          <button :class="{ active: mode === 'scroll' }" @click="mode = 'scroll'">滚动</button
          ><button :class="{ active: mode === 'page' }" @click="mode = 'page'">翻页</button>
        </div>
        <button class="icon-btn" title="沉浸模式 (F)" @click="zen = true">
          <AppIcon name="expand" :size="18" />
        </button>
      </div>
      <WindowControls />
    </header>
    <div class="reader-body">
      <div v-if="zen" class="reader-zen-titlebar" @mousedown="dragWindow">
        <button class="icon-btn" title="退出沉浸 (Esc)" @click="zen = false">
          <AppIcon name="collapse" :size="18" />
        </button>
        <WindowControls />
      </div>
      <div v-if="store.loading" class="loading"><span class="spinner" />正在加载章节图片…</div>
      <EmptyState
        v-else-if="store.error"
        error
        title="本章暂时没有加载成功"
        :description="store.error"
        ><button class="btn btn-primary" @click="load">重新加载</button
        ><button class="btn btn-secondary" @click="back">返回章节</button></EmptyState
      ><EmptyState
        v-else-if="!pictures.length"
        title="这一章还没有图片"
        description="返回章节列表，试试其他章节。"
        icon="image"
        ><button class="btn btn-secondary" @click="back">返回章节</button></EmptyState
      >
      <div v-else-if="mode === 'scroll'" ref="scrollContainer" class="scroll-container">
        <div
          v-for="(pic, idx) in pictures"
          :key="pic._id"
          class="scroll-image"
          :data-index="idx"
          :style="{ maxWidth: width + 'px' }"
        >
          <img
            v-if="!failed.has(pic._id)"
            :src="store.getImageUrl(pic)"
            :alt="'第 ' + (idx + 1) + ' 页'"
            :loading="idx < currentIndex + 2 ? 'eager' : 'lazy'"
            @error="failed.add(pic._id)"
          />
          <div v-else class="image-failed">
            第 {{ idx + 1 }} 页加载失败<button
              class="btn btn-secondary btn-sm"
              @click="retry(pic._id)"
            >
              重新加载图片
            </button>
          </div>
          <span class="page-number">{{ idx + 1 }} / {{ total }}</span>
        </div>
        <div class="load-more">
          <div v-if="store.fetchingPictures" class="reader-fetching">
            <span class="spinner" />正在加载本章剩余页面…
          </div>
          <div v-else-if="store.picturesError" role="alert" class="notice error reader-fetch-error">
            后续页面加载失败：{{ store.picturesError }}
          </div>
          <button
            v-if="nextEpisode"
            class="btn btn-secondary"
            @click="router.replace('/reader/' + comicId + '/' + nextEpisode._id)"
          >
            下一章 <AppIcon name="arrow" :size="15" /></button
          ><button v-else class="btn btn-secondary" @click="back">本章读完了 · 返回作品</button>
        </div>
      </div>
      <div v-else class="page-container">
        <button
          class="page-nav prev"
          title="上一页 (←)"
          :disabled="currentIndex === 0"
          @click="goTo(currentIndex - 1)"
        >
          <AppIcon name="chevron-left" :size="25" /></button
        ><template v-if="currentPic"
          ><img
            v-if="!failed.has(currentPic._id)"
            class="page-image"
            :src="store.getImageUrl(currentPic)"
            :alt="'第 ' + (currentIndex + 1) + ' 页'"
            @error="failed.add(currentPic._id)"
            @click="goTo(currentIndex + 1)"
          />
          <div v-else class="image-failed">
            图片加载失败<button class="btn btn-secondary" @click="retry(currentPic._id)">
              重新加载
            </button>
          </div></template
        ><button
          class="page-nav next"
          title="下一页 (→)"
          :disabled="currentIndex >= pictures.length - 1"
          @click="goTo(currentIndex + 1)"
        >
          <AppIcon name="chevron-right" :size="25" />
        </button>
      </div>
    </div>
    <footer class="reader-footer">
      <span
        >{{ pictures.length ? currentIndex + 1 : 0 }} / {{ total }} 页<span
          v-if="store.fetchingPictures"
          class="reader-footer-loading"
          ><span class="spinner" />加载中</span
        ></span
      ><input
        type="range"
        aria-label="跳转阅读页码"
        :min="0"
        :max="Math.max(0, total - 1)"
        :value="currentIndex"
        :disabled="!pictures.length"
        @input="goTo(Number(($event.target as HTMLInputElement).value))"
      /><span>{{ mode === 'page' ? '← → 翻页 · ' : '' }}F 沉浸 · Esc 返回</span>
    </footer>
  </div>
</template>

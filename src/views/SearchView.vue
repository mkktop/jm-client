<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import ComicCard from '../components/ComicCard.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import AppIcon from '../components/AppIcon.vue'
import Paginator from '../components/Paginator.vue'
import { readPreference, writePreference } from '../utils/readerPreferences'

defineOptions({ name: 'SearchView' })
const route = useRoute()
const router = useRouter()
const store = useComicStore()
const keyword = ref(String(route.query.q || ''))
const mainTag = ref<number>(String(route.query.type) === 'tag' ? 3 : 0)
const time = ref('a')
const sort = ref('mr')
const searched = ref(false)
const submittedKeyword = ref('')
const resultsTop = ref<HTMLElement>()
const modes = [
  { value: 0, label: '全站', hint: '标题 / 标签 / 作者' },
  { value: 1, label: '作品', hint: '作品名' },
  { value: 2, label: '作者', hint: '作者名' },
  { value: 3, label: '标签', hint: '精确标签' },
  { value: 4, label: '角色', hint: '角色名' }
]
const sortOptions = [
  { value: 'mr', label: '最新' },
  { value: 'mv', label: '浏览最多' },
  { value: 'mp', label: '图片最多' },
  { value: 'tf', label: '点赞最多' },
  { value: 'tr', label: '评分最高' },
  { value: 'md', label: '评论最多' }
]
const timeOptions = [
  { value: 'a', label: '全部时间' },
  { value: 't', label: '今天' },
  { value: 'w', label: '本周' },
  { value: 'm', label: '本月' }
]
const HISTORY_KEY = 'jm-search-history'
const history = ref<string[]>(readPreference<string[]>(HISTORY_KEY, []))
const modeLabel = computed(
  () => modes.find((mode) => mode.value === Number(mainTag.value))?.label || '全站'
)
function remember(value: string) {
  history.value = [value, ...history.value.filter((item) => item !== value)].slice(0, 10)
  writePreference(HISTORY_KEY, history.value)
}
function clearHistory() {
  history.value = []
  writePreference(HISTORY_KEY, history.value)
}
async function doSearch(page = 1) {
  const value = keyword.value.trim()
  if (!value) return
  searched.value = true
  submittedKeyword.value = value
  remember(value)
  await store.search(value, page, sort.value, Number(mainTag.value), time.value)
}
async function goToPage(page: number) {
  if (store.loading) return
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  await store.search(submittedKeyword.value, page, sort.value, Number(mainTag.value), time.value)
}
function retry() {
  store.search(
    submittedKeyword.value,
    store.searchPage,
    sort.value,
    Number(mainTag.value),
    time.value
  )
}
function chooseHistory(value: string) {
  keyword.value = value
  doSearch()
}
function switchMode(value: number) {
  if (mainTag.value === value) return
  mainTag.value = value
  if (searched.value) doSearch()
}
function changeSort() {
  if (searched.value) doSearch()
}
function changeTime() {
  if (searched.value) doSearch()
}
watch(
  () => [route.query.q, route.query.type],
  ([q, t]) => {
    if (!q) return
    const nextTag = String(t) === 'tag' ? 3 : 0
    const nextKeyword = String(q)
    if (nextKeyword !== keyword.value || nextTag !== mainTag.value) {
      keyword.value = nextKeyword
      mainTag.value = nextTag
      doSearch()
    }
  }
)
onMounted(() => {
  store.searchResults = []
  store.error = null
  if (keyword.value) doSearch()
})
</script>
<template>
  <div class="page search-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">SEARCH · JM COMICS</div>
        <h1>搜索与探索</h1>
        <p>支持全站、作品、作者、标签、角色搜索，以及 +标签 / -标签 语法。</p>
      </div>
      <router-link to="/categories" class="btn btn-secondary btn-sm"
        ><AppIcon name="grid" :size="14" />分类浏览</router-link
      >
    </div>
    <div class="search-modes" role="tablist" aria-label="搜索类型">
      <button
        v-for="mode in modes"
        :key="mode.value"
        type="button"
        role="tab"
        :aria-selected="Number(mainTag) === mode.value"
        :class="{ active: Number(mainTag) === mode.value }"
        :title="mode.hint"
        @click="switchMode(mode.value)"
      >
        {{ mode.label }}
      </button>
    </div>
    <form class="search-panel" @submit.prevent="doSearch(1)">
      <div class="search-field">
        <AppIcon name="search" :size="22" /><input
          v-model="keyword"
          aria-label="搜索关键词"
          :placeholder="
            Number(mainTag) === 3
              ? '输入标签，如：全彩、純愛、原神'
              : Number(mainTag) === 2
                ? '输入作者名'
                : Number(mainTag) === 4
                  ? '输入角色名'
                  : '输入漫画名、作者、标签或角色'
          "
        /><button
          v-if="keyword"
          type="button"
          title="清空关键词"
          class="icon-btn"
          @click="keyword = ''"
        >
          <AppIcon name="close" :size="16" />
        </button>
      </div>
      <select v-model="time" class="input search-sort" aria-label="时间范围" @change="changeTime">
        <option v-for="option in timeOptions" :key="option.value" :value="option.value">
          {{ option.label }}
        </option>
      </select>
      <select v-model="sort" class="input search-sort" aria-label="搜索排序" @change="changeSort">
        <option v-for="option in sortOptions" :key="option.value" :value="option.value">
          {{ option.label }}
        </option>
      </select>
      <button class="btn btn-primary" :disabled="store.loading || !keyword.trim()">
        搜索漫画 <AppIcon name="arrow" :size="15" />
      </button>
    </form>
    <div class="hot-keywords">
      <span><AppIcon name="layers" :size="15" /> 语法</span>
      <span class="search-syntax"
        >裸词 OR · <code>+标签</code> AND · <code>-标签</code> 排除 · 服务端自动简繁归一化</span
      >
    </div>
    <div v-if="history.length" class="hot-keywords">
      <span><AppIcon name="clock" :size="15" /> 最近搜索</span
      ><button
        v-for="value in history"
        :key="value"
        class="hot-tag"
        :disabled="store.loading"
        @click="chooseHistory(value)"
      >
        {{ value }}
      </button><button class="text-btn tag-history-clear" :disabled="store.loading" @click="clearHistory">
        清除
      </button>
    </div>
    <div v-if="searched" ref="resultsTop" class="section-heading search-results-heading">
      <h2>
        搜索结果<small v-if="!store.loading"
          >第 {{ store.searchPage }} 页 · {{ store.searchTotal }} 条</small
        >
      </h2>
      <span class="header-note">{{ modeLabel }}模式 · 关键词「{{ submittedKeyword }}」</span>
    </div>
    <ComicSkeleton v-if="store.loading && !store.searchResults.length" />
    <EmptyState
      v-else-if="store.error"
      title="搜索暂时遇到了一点问题"
      :description="store.error"
      error
      ><button class="btn btn-primary" @click="retry">重新搜索</button></EmptyState
    >
    <template v-else-if="store.searchResults.length">
      <div class="comic-grid">
        <ComicCard
          v-for="comic in store.searchResults"
          :key="comic._id"
          :comic="comic"
          @click="router.push('/comic/' + comic._id)"
        />
      </div>
      <Paginator
        :page="store.searchPage"
        :pages="store.searchPages"
        :loading="store.loading"
        @change="goToPage"
      />
    </template>
    <EmptyState
      v-else-if="searched"
      title="没有找到相关漫画"
      description="试试更短的关键词、换个搜索类型，或用 +标签 组合。"
      icon="search"
    />
    <div v-else class="search-welcome">
      <div class="search-welcome-art"><AppIcon name="search" :size="42" /><span>?</span></div>
      <h2>想看什么，搜一下。</h2>
      <p>按全站、作品、作者、标签或角色搜索；也可以用 +标签 精确组合。</p>
      <span class="eyebrow">SEARCH BY TITLE · AUTHOR · TAG · CHARACTER</span>
    </div>
  </div>
</template>
<style scoped>
.search-syntax {
  font-size: 11px;
  color: var(--text-secondary);
}
.search-syntax code {
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
}
</style>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import ComicCard from '../components/ComicCard.vue'
import AppIcon from '../components/AppIcon.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import Paginator from '../components/Paginator.vue'

defineOptions({ name: 'WeeklyView' })
const store = useComicStore()
const route = useRoute()
const router = useRouter()
const filter = ref('')
const resultsTop = ref<HTMLElement>()
const TYPE_LABELS: Record<string, string> = { manga: '日漫', another: '其他', hanman: '韩漫' }
interface WeeklyIssue {
  id: string
  title: string
  time: string
}
interface IssueParts {
  primary: string
  secondary: string
  full: string
}
function issueParts(issue: WeeklyIssue): IssueParts {
  // time 形如「2026第263期10.30 - 10.23」，近几期 title 为空，需要从中拆出期号。
  const match = issue.time.trim().match(/^(\d{4})\s*第\s*(\d+)\s*期\s*(.*)$/)
  const year = match?.[1] || ''
  const number = match?.[2] || ''
  const range = (match?.[3] || '').replace(/\s*-\s*/g, ' – ').trim()
  const title = issue.title.trim()
  const primary = title || (number ? `第${number}期` : issue.time || '未命名期刊')
  const issueTag = year && number ? `${year}第${number}期` : number ? `第${number}期` : year
  const secondary = [title ? issueTag : year ? `${year} 年` : '', range]
    .filter(Boolean)
    .join(' · ')
  const full = [primary, secondary].filter(Boolean).join(' · ')
  return { primary, secondary, full }
}
function typeLabel(id: string, title: string): string {
  return title.trim() || TYPE_LABELS[id] || id
}
const selected = computed(() =>
  typeof route.query.issue === 'string' ? route.query.issue : store.weeklyCategoryId
)
const currentType = computed(() =>
  typeof route.query.type === 'string' ? route.query.type : store.weeklyTypeId
)
const currentIssue = computed(
  () => store.weeklyInfo?.categories.find((item) => item.id === selected.value) || null
)
const currentIssueParts = computed(() => (currentIssue.value ? issueParts(currentIssue.value) : null))
const currentTypeTitle = computed(() => {
  const type = store.weeklyInfo?.types.find((item) => item.id === currentType.value)
  return type ? typeLabel(type.id, type.title) : currentType.value
})
const weeklyInfoLoading = computed(() => !!store.loadingKeys.weeklyInfo)
const weeklyLoading = computed(() => !!store.loadingKeys.weekly)
const weeklyInfoError = computed(() => store.errorKeys.weeklyInfo || null)
const weeklyError = computed(() => store.errorKeys.weekly || null)
const issues = computed(() => {
  const term = filter.value.trim().toLowerCase()
  return (store.weeklyInfo?.categories || [])
    .map((issue) => ({ issue, parts: issueParts(issue) }))
    .filter(
      ({ issue, parts }) =>
        !term ||
        issue.title.toLowerCase().includes(term) ||
        issue.time.toLowerCase().includes(term) ||
        parts.primary.toLowerCase().includes(term) ||
        parts.secondary.toLowerCase().includes(term)
    )
})
function chooseIssue(id: string) {
  router.push({ name: 'weekly', query: { ...route.query, issue: id } })
}
function chooseType(id: string) {
  router.push({ name: 'weekly', query: { ...route.query, type: id } })
}
function goToPage(page: number) {
  if (weeklyLoading.value) return
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  store.fetchWeekly(page, selected.value, currentType.value)
}
watch(
  [selected, currentType],
  () => {
    if (selected.value) {
      store.fetchWeekly(1, selected.value, currentType.value)
    }
  },
  { immediate: true }
)
watch(
  () => store.weeklyInfo,
  (info) => {
    if (!info) return
    const hasIssue = info.categories.some((item) => item.id === selected.value)
    if (!hasIssue && info.categories.length) {
      router.replace({ name: 'weekly', query: { ...route.query, issue: info.categories[0].id } })
    }
    const hasType = info.types.some((item) => item.id === currentType.value)
    if (!hasType && info.types.length) {
      router.replace({ name: 'weekly', query: { ...route.query, type: info.types[0].id } })
    }
  }
)
onMounted(() => store.fetchWeeklyInfo())
</script>
<template>
  <div class="page weekly-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">WEEKLY · JM PICKS</div>
        <h1>每周必看</h1>
        <p>按期刊浏览 JM 的每周推荐，支持日漫 / 其他 / 韩漫。</p>
      </div>
      <button
        class="btn btn-secondary btn-sm"
        :disabled="weeklyInfoLoading"
        @click="store.fetchWeeklyInfo()"
      >
        <AppIcon name="refresh" :size="14" />刷新期刊
      </button>
    </div>
    <section class="category-panel" :class="{ 'has-selection': !!currentIssue }" aria-label="期刊列表">
      <div class="category-panel-heading">
        <strong
          >全部期刊 <span v-if="store.weeklyInfo">{{ store.weeklyInfo.categories.length }}</span></strong
        ><label class="category-filter"
          ><AppIcon name="search" :size="15" /><input
            v-model="filter"
            aria-label="筛选期刊"
            placeholder="筛选期刊标题或日期" /><button
            v-if="filter"
            class="icon-btn"
            title="清空筛选"
            @click="filter = ''"
          >
            <AppIcon name="close" :size="13" /></button
        ></label>
      </div>
      <div v-if="weeklyInfoLoading && !store.weeklyInfo" class="category-grid" aria-busy="true">
        <span v-for="n in 12" :key="n" class="skeleton category-skeleton" />
      </div>
      <EmptyState
        v-else-if="weeklyInfoError && !store.weeklyInfo"
        error
        title="期刊加载失败"
        :description="weeklyInfoError"
        ><button class="btn btn-primary" @click="store.fetchWeeklyInfo()">重新加载</button></EmptyState
      >
      <div v-else-if="issues.length" class="category-grid">
        <button
          v-for="{ issue, parts } in issues"
          :key="issue.id"
          class="category-option"
          :class="{ active: currentIssue?.id === issue.id }"
          :aria-pressed="currentIssue?.id === issue.id"
          :title="parts.full"
          @click="chooseIssue(issue.id)"
        >
          <AppIcon name="clock" :size="15" /><span class="category-option-text"
            ><strong>{{ parts.primary }}</strong
            ><small v-if="parts.secondary">{{ parts.secondary }}</small></span
          >
        </button>
      </div>
      <p v-else class="category-no-match">
        {{ filter ? '没有匹配的期刊。' : '暂无可浏览的期刊。' }}
      </p>
    </section>
    <template v-if="currentIssue">
      <div ref="resultsTop" class="section-heading category-results-heading">
        <div>
          <h2>
            {{ currentIssueParts?.primary }}
            <small v-if="!weeklyLoading">{{ store.weeklyList.length }} 部</small>
          </h2>
          <p class="category-aliases">
            {{ currentIssueParts?.secondary }}<template v-if="currentTypeTitle">
              · {{ currentTypeTitle }}</template
            >
          </p>
        </div>
        <div class="section-controls">
          <div class="segmented">
            <button
              v-for="type in store.weeklyInfo?.types || []"
              :key="type.id"
              :class="{ active: currentType === type.id }"
              :disabled="weeklyLoading"
              @click="chooseType(type.id)"
            >
              {{ typeLabel(type.id, type.title) }}
            </button>
          </div>
        </div>
      </div>
      <ComicSkeleton v-if="weeklyLoading && !store.weeklyList.length" />
      <EmptyState
        v-else-if="weeklyError && !store.weeklyList.length"
        error
        title="期刊内容加载失败"
        :description="weeklyError"
        ><button class="btn btn-primary" @click="store.fetchWeekly(1, selected, currentType)">
          重新加载
        </button></EmptyState
      >
      <div v-else-if="store.weeklyList.length" class="comic-grid">
        <ComicCard
          v-for="comic in store.weeklyList"
          :key="comic._id"
          :comic="comic"
          @click="router.push('/comic/' + comic._id)"
        />
      </div>
      <EmptyState
        v-else
        title="这一期暂无内容"
        description="换一期或换一个类型看看。"
        icon="clock"
      />
      <Paginator
        v-if="!weeklyError && store.weeklyList.length"
        :page="store.weeklyPage"
        :pages="store.weeklyPages"
        :loading="weeklyLoading"
        @change="goToPage"
      />
    </template>
  </div>
</template>

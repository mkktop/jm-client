<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useComicStore } from '../stores/comic'
import { useAppStore } from '../stores/app'
import ComicCard from '../components/ComicCard.vue'
import AppIcon from '../components/AppIcon.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import Paginator from '../components/Paginator.vue'
import type { Comic, PromoteList, PromoteSection } from '../types'

const router = useRouter()
defineOptions({ name: 'HomeView' })
const store = useComicStore()
const app = useAppStore()
const resultsTop = ref<HTMLElement>()
const periods = [
  { value: 'mv_t', label: '今日' },
  { value: 'mv_w', label: '本周' },
  { value: 'mv_m', label: '本月' }
]
const selectedSection = ref<PromoteSection | null>(null)
const sectionItems = ref<Comic[]>([])
const sectionPage = ref(1)
const sectionTotal = ref(0)
const sectionLoading = ref(false)

async function load() {
  // 各区块独立请求、并行加载；周刊索引与期刊内容有依赖，放在同一分支里串行。
  const weekly = (async () => {
    await store.fetchWeeklyInfo()
    if (store.weeklyCategoryId) {
      await store.fetchWeekly(1, store.weeklyCategoryId, store.weeklyTypeId)
    }
  })()
  await Promise.allSettled([
    store.fetchPromote(),
    store.fetchRanking(),
    store.fetchLatest(1),
    weekly
  ])
}
const homeLoading = computed(() =>
  ['promote', 'ranking', 'latest', 'weeklyInfo', 'weekly'].some(
    (key) => store.loadingKeys[key]
  )
)
const homeError = computed(
  () =>
    store.errorKeys.promote ||
    store.errorKeys.ranking ||
    store.errorKeys.latest ||
    store.errorKeys.weeklyInfo ||
    store.errorKeys.weekly ||
    null
)
async function changePeriod(period: string) {
  if (store.loadingKeys.ranking) return
  await store.fetchRanking(period)
}
function goToLatestPage(page: number) {
  if (store.loadingKeys.latest) return
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  store.fetchLatest(page)
}
async function openSection(section: PromoteSection) {
  selectedSection.value = section
  sectionItems.value = section.content
  sectionPage.value = 1
  sectionTotal.value = section.content.length
  sectionLoading.value = true
  try {
    const page = await invoke<PromoteList>('get_promote_list', {
      sectionId: section.id,
      page: 1
    })
    sectionItems.value = page.list
    sectionTotal.value = page.total
  } catch {
    // 分区详情失败时保留 /promote 自带内容
  } finally {
    sectionLoading.value = false
  }
}
async function loadMoreSection() {
  if (!selectedSection.value || sectionLoading.value) return
  sectionLoading.value = true
  try {
    const page = sectionPage.value + 1
    const data = await invoke<PromoteList>('get_promote_list', {
      sectionId: selectedSection.value.id,
      page
    })
    sectionItems.value = sectionItems.value.concat(data.list)
    sectionPage.value = page
  } finally {
    sectionLoading.value = false
  }
}
async function changeWeeklyType(typeId: string) {
  await store.fetchWeekly(1, store.weeklyCategoryId, typeId)
}
onMounted(load)
</script>
<template>
  <div class="page home-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">JM · MANGA DISCOVERY</div>
        <h1>发现漫画</h1>
        <p>推荐位、排行榜、最新更新与本周必看。</p>
      </div>
      <div class="section-controls">
        <router-link to="/ranking" class="btn btn-secondary btn-sm"
          ><AppIcon name="trend" :size="14" />完整排行榜</router-link
        >
        <button
          class="icon-btn"
          :title="app.compactCards ? '使用舒展布局' : '使用紧凑布局'"
          @click="app.toggleCompactCards"
        >
          <AppIcon name="grid" :size="16" />
        </button>
      </div>
    </div>

    <ComicSkeleton v-if="homeLoading && !store.promote.length && !store.ranking.length" />
    <EmptyState
      v-else-if="homeError && !store.promote.length && !store.ranking.length"
      title="内容加载失败"
      :description="homeError"
      error
      ><button class="btn btn-primary" @click="load">
        <AppIcon name="refresh" :size="15" />重新加载</button
      ><router-link class="btn btn-secondary" to="/settings">检查网络设置</router-link></EmptyState
    >

    <template v-else>
      <template v-for="section in store.promote" :key="section.id">
        <section v-if="section.content.length" class="promote-section">
          <div class="section-heading">
            <h2>{{ section.title || '推荐位' }}</h2>
            <div class="section-controls">
              <button class="text-btn" @click="openSection(section)">
                查看全部 <AppIcon name="arrow" :size="13" />
              </button>
            </div>
          </div>
          <div class="comic-grid">
            <ComicCard
              v-for="comic in section.content.slice(0, 8)"
              :key="comic._id"
              :comic="comic"
              @click="router.push('/comic/' + comic._id)"
            />
          </div>
        </section>
      </template>

      <section v-if="selectedSection" class="promote-section">
        <div class="section-heading">
          <h2>{{ selectedSection.title || '推荐位' }}<small>共 {{ sectionTotal }} 部</small></h2>
          <div class="section-controls">
            <button class="text-btn" @click="selectedSection = null">收起</button>
          </div>
        </div>
        <div class="comic-grid">
          <ComicCard
            v-for="comic in sectionItems"
            :key="comic._id"
            :comic="comic"
            @click="router.push('/comic/' + comic._id)"
          />
        </div>
        <div v-if="sectionItems.length < sectionTotal" class="load-more">
          <button class="btn btn-secondary" :disabled="sectionLoading" @click="loadMoreSection">
            <span v-if="sectionLoading" class="spinner" />加载更多
          </button>
        </div>
      </section>

      <div ref="resultsTop" class="section-heading">
        <h2>热门排行榜<small>DAILY / WEEKLY / MONTHLY</small></h2>
        <div class="section-controls">
          <div class="segmented">
            <button
              v-for="period in periods"
              :key="period.value"
              :class="{ active: store.rankingPeriod === period.value }"
              :disabled="store.loadingKeys.ranking"
              @click="changePeriod(period.value)"
            >
              {{ period.label }}
            </button>
          </div>
        </div>
      </div>
      <ComicSkeleton v-if="store.loadingKeys.ranking && !store.ranking.length" />
      <div v-else-if="store.ranking.length" class="comic-grid">
        <ComicCard
          v-for="(comic, index) in store.ranking.slice(0, 12)"
          :key="comic._id"
          :comic="comic"
          :rank="index + 1"
          @click="router.push('/comic/' + comic._id)"
        />
      </div>
      <EmptyState v-else title="排行榜暂无内容" description="稍后刷新，或去搜索看看。" icon="trend" />

      <section class="section-heading">
        <h2>最新更新<small>LATEST UPDATES</small></h2>
        <div class="section-controls">
          <router-link to="/categories" class="text-btn"
            >按分类找本 <AppIcon name="arrow" :size="13"
          /></router-link>
        </div>
      </section>
      <ComicSkeleton v-if="store.loadingKeys.latest && !store.latest.length" />
      <div v-else-if="store.latest.length" class="comic-grid">
        <ComicCard
          v-for="comic in store.latest"
          :key="comic._id"
          :comic="comic"
          @click="router.push('/comic/' + comic._id)"
        />
      </div>
      <EmptyState
        v-else
        title="暂无最新更新"
        description="可以稍后刷新，或直接搜索作品。"
        icon="book"
        ><router-link to="/search" class="btn btn-primary">去搜索</router-link></EmptyState
      >
      <Paginator
        v-if="store.latest.length"
        :page="store.latestPage"
        :pages="store.latestPages"
        :loading="store.loadingKeys.latest"
        @change="goToLatestPage"
      />

      <section class="section-heading">
        <h2>本周必看<small>WEEKLY PICKS</small></h2>
        <div class="section-controls">
          <div class="segmented">
            <button
              v-for="type in store.weeklyInfo?.types || []"
              :key="type.id"
              :class="{ active: store.weeklyTypeId === type.id }"
              :disabled="store.loadingKeys.weekly"
              @click="changeWeeklyType(type.id)"
            >
              {{ type.title || type.id }}
            </button>
          </div>
          <router-link to="/weekly" class="text-btn"
            >往期期刊 <AppIcon name="arrow" :size="13"
          /></router-link>
        </div>
      </section>
      <ComicSkeleton v-if="store.loadingKeys.weekly && !store.weeklyList.length" />
      <div v-else-if="store.weeklyList.length" class="comic-grid">
        <ComicCard
          v-for="comic in store.weeklyList.slice(0, 10)"
          :key="comic._id"
          :comic="comic"
          @click="router.push('/comic/' + comic._id)"
        />
      </div>
      <EmptyState
        v-else
        title="本周必看暂无内容"
        description="去往期期刊看看。"
        icon="clock"
        ><router-link to="/weekly" class="btn btn-secondary">往期期刊</router-link></EmptyState
      >

      <footer class="page-footer">
        <span>JM · 禁漫天堂桌面客户端</span><span>BROWSE · COLLECT · DOWNLOAD</span>
      </footer>
    </template>
  </div>
</template>
<style scoped>
.promote-section {
  margin-bottom: 30px;
}
.load-more {
  display: flex;
  justify-content: center;
  padding: 16px 0;
}
</style>

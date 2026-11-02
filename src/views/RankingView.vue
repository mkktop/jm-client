<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import ComicCard from '../components/ComicCard.vue'
import AppIcon from '../components/AppIcon.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'

defineOptions({ name: 'RankingView' })
const router = useRouter()
const store = useComicStore()
const category = ref('')
const periods = [
  { value: 'mv_t', label: '今日榜' },
  { value: 'mv_w', label: '本周榜' },
  { value: 'mv_m', label: '本月榜' }
]
async function load() {
  if (!store.categories.length) await store.fetchCategories()
  await store.fetchRanking(store.rankingPeriod, category.value)
}
async function changePeriod(period: string) {
  if (store.loadingKeys.ranking) return
  await store.fetchRanking(period, category.value)
}
async function changeCategory() {
  if (store.loadingKeys.ranking) return
  await store.fetchRanking(store.rankingPeriod, category.value)
}
onMounted(load)
</script>
<template>
  <div class="page ranking-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">RANKING · JM COMICS</div>
        <h1>排行榜</h1>
        <p>日榜、周榜、月榜，支持按分类筛选。</p>
      </div>
      <button
        class="btn btn-secondary btn-sm"
        :disabled="store.loadingKeys.ranking"
        @click="load"
      >
        <AppIcon name="refresh" :size="14" />刷新
      </button>
    </div>
    <div class="section-heading">
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
      <div class="section-controls">
        <select v-model="category" class="input category-sort" @change="changeCategory">
          <option value="">全部分类</option>
          <option v-for="item in store.categories" :key="item.slug" :value="item.slug">
            {{ item.title }}
          </option>
        </select>
      </div>
    </div>
    <ComicSkeleton v-if="store.loadingKeys.ranking && !store.ranking.length" />
    <EmptyState
      v-else-if="store.errorKeys.ranking"
      error
      title="排行榜加载失败"
      :description="store.errorKeys.ranking || ''"
      ><button class="btn btn-primary" @click="load">重新加载</button></EmptyState
    >
    <div v-else-if="store.ranking.length" class="comic-grid">
      <ComicCard
        v-for="(comic, index) in store.ranking"
        :key="comic._id"
        :comic="comic"
        :rank="index + 1"
        @click="router.push('/comic/' + comic._id)"
      />
    </div>
    <EmptyState
      v-else
      title="这个榜单暂无内容"
      description="换一个时间范围或分类试试。"
      icon="trend"
    />
  </div>
</template>

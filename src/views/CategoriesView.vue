<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import AppIcon from '../components/AppIcon.vue'
import ComicCard from '../components/ComicCard.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import Paginator from '../components/Paginator.vue'
import type { Category } from '../types'

defineOptions({ name: 'CategoriesView' })
const store = useComicStore()
const route = useRoute()
const router = useRouter()
const filter = ref('')
const resultsTop = ref<HTMLElement>()
const order = ref('mr')
const time = ref('a')
const orders = [
  { value: 'mr', label: '最新' },
  { value: 'mv', label: '浏览最多' },
  { value: 'mp', label: '图片最多' },
  { value: 'tf', label: '点赞最多' },
  { value: 'tr', label: '评分最高' },
  { value: 'md', label: '评论最多' }
]
const times = [
  { value: 'a', label: '全部时间' },
  { value: 't', label: '今天' },
  { value: 'w', label: '本周' },
  { value: 'm', label: '本月' }
]
const selected = computed(() =>
  typeof route.query.category === 'string' ? route.query.category.trim() : ''
)
const current = computed<Category | null>(
  () => store.categories.find((item) => item.slug === selected.value) || null
)
const groups = computed(() => {
  const term = filter.value.trim().toLowerCase()
  if (!term) return store.categories
  return store.categories.filter(
    (item) =>
      item.title.toLowerCase().includes(term) || item.slug.toLowerCase().includes(term)
  )
})
function choose(category?: Category) {
  router.push({
    name: 'categories',
    query: category ? { ...route.query, category: category.slug } : {}
  })
}
function searchSub(title: string) {
  router.push({ name: 'search', query: { q: title, type: 'tag' } })
}
function goToPage(page: number) {
  if (!current.value || store.loading) return
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  store.fetchCategoryComics(page, current.value.slug, order.value, time.value)
}
function retry() {
  if (current.value) {
    store.fetchCategoryComics(
      store.categoryComics.length ? store.categoryPage + 1 : 1,
      current.value.slug,
      order.value,
      time.value
    )
  }
}
watch(
  [selected, order, time],
  () => {
    if (selected.value) {
      store.fetchCategoryComics(1, selected.value, order.value, time.value)
    }
  },
  { immediate: true }
)
watch(
  () => store.categories,
  (list) => {
    if (!selected.value && list.length) {
      router.replace({ name: 'categories', query: { category: list[0].slug } })
    }
  }
)
onMounted(() => store.fetchCategories())
</script>
<template>
  <div class="page categories-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">CATEGORIES</div>
        <h1>分类浏览</h1>
        <p>按 JM 站点分类找本，支持时间与排序筛选。</p>
      </div>
      <button
        class="btn btn-secondary btn-sm"
        :disabled="store.categoriesLoading || store.loading"
        @click="store.fetchCategories()"
      >
        <AppIcon name="refresh" :size="14" />刷新分类
      </button>
    </div>
    <section class="category-panel" :class="{ 'has-selection': !!selected }" aria-label="站点分类">
      <div class="category-panel-heading">
        <strong>全部分类 <span v-if="store.categories.length">{{ store.categories.length }}</span></strong
        ><label class="category-filter"
          ><AppIcon name="search" :size="15" /><input
            v-model="filter"
            aria-label="筛选分类"
            placeholder="筛选分类名称" /><button
            v-if="filter"
            class="icon-btn"
            title="清空分类筛选"
            @click="filter = ''"
          >
            <AppIcon name="close" :size="13" /></button
        ></label>
      </div>
      <div
        v-if="store.categoriesLoading && !store.categories.length"
        class="category-grid"
        aria-busy="true"
      >
        <span v-for="n in 10" :key="n" class="skeleton category-skeleton" />
      </div>
      <EmptyState
        v-else-if="store.error && !store.categories.length"
        error
        title="分类加载失败"
        :description="store.error"
        ><button class="btn btn-primary" @click="store.fetchCategories()">重新加载分类</button></EmptyState
      >
      <div v-else-if="groups.length" class="category-grid">
        <button
          v-for="category in groups"
          :key="category.slug"
          class="category-option"
          :class="{ active: current?.slug === category.slug }"
          :aria-pressed="current?.slug === category.slug"
          :title="category.title + ' · ' + category.totalAlbums + ' 部'"
          @click="choose(category)"
        >
          <AppIcon name="grid" :size="15" /><span>{{ category.title }}</span
          ><small>{{ category.totalAlbums.toLocaleString() }}</small>
        </button>
      </div>
      <p v-else class="category-no-match">
        {{ filter ? '没有匹配的分类，试试其他名称。' : '暂无可浏览的分类。' }}
      </p>
    </section>
    <template v-if="current">
      <div ref="resultsTop" class="section-heading category-results-heading">
        <div>
          <h2>
            {{ current.title
            }}<small v-if="!store.loading && store.categoryPages > 1"
              >第 {{ store.categoryPage }} / {{ store.categoryPages }} 页</small
            >
          </h2>
          <p class="category-aliases">
            共 {{ current.totalAlbums.toLocaleString() }} 部 ·
            {{ current.subCategories.length }} 个子分类
          </p>
        </div>
        <div class="section-controls">
          <button class="text-btn" @click="choose()">全部分类</button
          ><select v-model="order" class="input category-sort" aria-label="排序">
            <option v-for="option in orders" :key="option.value" :value="option.value">
              {{ option.label }}
            </option></select
          ><select v-model="time" class="input category-sort" aria-label="时间">
            <option v-for="option in times" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>
      </div>
      <div v-if="current.subCategories.length" class="hot-keywords">
        <span><AppIcon name="layers" :size="15" /> 子分类</span
        ><button
          v-for="sub in current.subCategories"
          :key="sub.slug"
          class="hot-tag"
          :disabled="store.loading"
          @click="searchSub(sub.title)"
        >
          {{ sub.title }}
        </button>
      </div>
      <ComicSkeleton v-if="store.loading && !store.categoryComics.length" />
      <template v-else>
        <div
          v-if="store.error && store.categoryComics.length"
          role="alert"
          class="notice error category-load-error"
        >
          {{ store.error }}<button class="text-btn" @click="retry">重试本页</button>
        </div>
        <EmptyState
          v-if="store.error && !store.categoryComics.length"
          error
          title="分类漫画加载失败"
          :description="store.error"
          ><button class="btn btn-primary" @click="retry">重新加载漫画</button></EmptyState
        >
        <div v-else-if="store.categoryComics.length" class="comic-grid">
          <ComicCard
            v-for="comic in store.categoryComics"
            :key="comic._id"
            :comic="comic"
            @click="router.push('/comic/' + comic._id)"
          />
        </div>
        <EmptyState
          v-else
          title="这个分类下暂无漫画"
          description="可以切换其他分类，或按关键词搜索。"
          icon="grid"
          ><router-link
            class="btn btn-secondary"
            :to="{ name: 'search', query: { q: current.title } }"
            >关键词搜索</router-link
          ></EmptyState
        >
        <Paginator
          v-if="!store.error"
          :page="store.categoryPage"
          :pages="store.categoryPages"
          :loading="store.loading"
          @change="goToPage"
        />
      </template>
    </template>
  </div>
</template>

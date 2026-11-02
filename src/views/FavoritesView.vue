<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '../stores/app'
import { useComicStore } from '../stores/comic'
import ComicCard from '../components/ComicCard.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import AppIcon from '../components/AppIcon.vue'
import Paginator from '../components/Paginator.vue'

defineOptions({ name: 'FavoritesView' })
const router = useRouter()
const app = useAppStore()
const store = useComicStore()
const resultsTop = ref<HTMLElement>()
const folderId = ref('0')
const sort = ref('mr')
async function load() {
  if (!app.isLoggedIn) return
  await store.fetchFavourites(1, sort.value, folderId.value)
}
function goToPage(page: number) {
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  store.fetchFavourites(page, sort.value, folderId.value)
}
function changeSort(value: string) {
  sort.value = value
  load()
}
function changeFolder() {
  load()
}
async function goLogin() {
  await app.logout()
}
onMounted(load)
</script>
<template>
  <div class="page favourites-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">YOUR JM COLLECTION</div>
        <h1>我的收藏</h1>
        <p>管理 JM 账号中的收藏与分组。</p>
      </div>
      <div v-if="app.isLoggedIn" class="section-controls">
        <select v-model="folderId" class="input category-sort" aria-label="收藏分组" @change="changeFolder">
          <option value="0">全部分组</option>
          <option v-for="folder in store.favouriteFolders" :key="folder._id" :value="folder._id">
            {{ folder.name }}
          </option>
        </select>
        <div class="segmented">
          <button
            :class="{ active: sort === 'mr' }"
            :disabled="store.loading"
            @click="changeSort('mr')"
          >
            最近收藏</button
          ><button
            :class="{ active: sort === 'mp' }"
            :disabled="store.loading"
            @click="changeSort('mp')"
          >
            最近更新
          </button>
        </div>
      </div>
    </div>
    <EmptyState
      v-if="!app.isLoggedIn"
      title="登录后查看收藏"
      description="JM 收藏夹需要账号登录；浏览和下载仍可使用游客模式。"
      icon="heart"
      ><button class="btn btn-primary" @click="goLogin">去登录</button
      ><router-link to="/" class="btn btn-secondary">继续浏览</router-link></EmptyState
    >
    <template v-else>
      <div ref="resultsTop" class="collection-banner">
        <span class="collection-banner-icon"><AppIcon name="heart" :size="24" /></span>
        <div>
          <strong>我的收藏书架</strong>
          <p>按收藏时间或更新时间查看，支持分组筛选。</p>
        </div>
        <span v-if="!store.loading" class="collection-count"
          >{{ store.favourites.length }} <small>部作品 · 当前页</small></span
        >
      </div>
      <ComicSkeleton v-if="store.loading && !store.favourites.length" />
      <EmptyState
        v-else-if="store.error"
        error
        title="收藏暂时没有加载成功"
        :description="store.error"
        ><button class="btn btn-primary" @click="load">重新加载</button></EmptyState
      >
      <EmptyState
        v-else-if="!store.favourites.length"
        title="还没有收藏漫画"
        description="在漫画详情页点击收藏，就能在这里找到它。"
        icon="heart"
        ><router-link to="/" class="btn btn-primary"
          >去找本子 <AppIcon name="arrow" :size="15" /></router-link
      ></EmptyState>
      <template v-else>
        <div class="comic-grid">
          <ComicCard
            v-for="comic in store.favourites"
            :key="comic._id"
            :comic="comic"
            @click="router.push('/comic/' + comic._id)"
          />
        </div>
        <Paginator
          :page="store.favouritePage"
          :pages="store.favouritePages"
          :loading="store.loading"
          @change="goToPage"
        />
      </template>
    </template>
  </div>
</template>

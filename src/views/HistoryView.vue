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

defineOptions({ name: 'HistoryView' })
const router = useRouter()
const app = useAppStore()
const store = useComicStore()
const resultsTop = ref<HTMLElement>()
async function load() {
  if (!app.isLoggedIn) return
  await store.fetchWatchList(1)
}
function goToPage(page: number) {
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  store.fetchWatchList(page)
}
async function goLogin() {
  await app.logout()
}
onMounted(load)
</script>
<template>
  <div class="page history-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">WATCH HISTORY</div>
        <h1>观看历史</h1>
        <p>JM 账号的观看记录；本地阅读进度仍会保存在本机。</p>
      </div>
      <button v-if="app.isLoggedIn" class="btn btn-secondary btn-sm" :disabled="store.loading" @click="load">
        <AppIcon name="refresh" :size="14" />刷新
      </button>
    </div>
    <EmptyState
      v-if="!app.isLoggedIn"
      title="登录后查看观看历史"
      description="观看历史需要 JM 账号；游客可以直接搜索和阅读。"
      icon="clock"
      ><button class="btn btn-primary" @click="goLogin">去登录</button
      ><router-link to="/" class="btn btn-secondary">继续浏览</router-link></EmptyState
    >
    <template v-else>
      <div ref="resultsTop" class="collection-banner">
        <span class="collection-banner-icon"><AppIcon name="clock" :size="24" /></span>
        <div>
          <strong>最近观看</strong>
          <p>从上次停下的地方继续。</p>
        </div>
        <span v-if="!store.loading" class="collection-count"
          >{{ store.watchList.length }} <small>部作品 · 当前页</small></span
        >
      </div>
      <ComicSkeleton v-if="store.loading && !store.watchList.length" />
      <EmptyState
        v-else-if="store.error"
        error
        title="观看历史加载失败"
        :description="store.error"
        ><button class="btn btn-primary" @click="load">重新加载</button></EmptyState
      >
      <EmptyState
        v-else-if="!store.watchList.length"
        title="还没有观看记录"
        description="读过的作品会出现在这里。"
        icon="clock"
        ><router-link to="/" class="btn btn-primary">去发现漫画</router-link></EmptyState
      >
      <template v-else>
        <div class="comic-grid">
          <ComicCard
            v-for="comic in store.watchList"
            :key="comic._id"
            :comic="comic"
            @click="router.push('/comic/' + comic._id)"
          />
        </div>
        <Paginator
          :page="store.watchPage"
          :pages="store.watchPages"
          :loading="store.loading"
          @change="goToPage"
        />
      </template>
    </template>
  </div>
</template>

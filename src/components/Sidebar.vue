<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore } from '../stores/app'
import { useComicStore } from '../stores/comic'
import AppIcon from './AppIcon.vue'
import BrandMark from './BrandMark.vue'
const app = useAppStore()
const comics = useComicStore()
const explore = [
  { path: '/', icon: 'compass', label: '发现漫画' },
  { path: '/categories', icon: 'grid', label: '分类浏览' },
  { path: '/ranking', icon: 'trend', label: '排行榜' },
  { path: '/weekly', icon: 'clock', label: '每周必看' },
  { path: '/search', icon: 'search', label: '搜索与探索' }
]
const personal = [
  { path: '/favorites', icon: 'heart', label: '我的收藏' },
  { path: '/history', icon: 'clock', label: '观看历史' },
  { path: '/library', icon: 'library', label: '本地书库' },
  { path: '/downloads', icon: 'download', label: '下载管理' }
]
const punching = ref(false)
const punchNotice = ref('')
async function punchIn() {
  if (punching.value || !app.isLoggedIn) return
  punching.value = true
  punchNotice.value = ''
  try {
    const result = await comics.checkin()
    punchNotice.value =
      result.message || (result.status === 'ok' ? '签到成功' : '签到已处理')
  } catch (e) {
    punchNotice.value = '签到失败：' + e
  } finally {
    punching.value = false
    setTimeout(() => (punchNotice.value = ''), 4000)
  }
}
async function goLogin() {
  await app.logout()
}
onMounted(() => {
  if (app.isLoggedIn) {
    comics.fetchProfile()
    comics.fetchDaily()
  }
})
</script>
<template>
  <aside class="sidebar" :class="{ collapsed: app.sidebarCollapsed }">
    <router-link to="/" class="sidebar-brand" title="JM 首页">
      <BrandMark />
      <span class="brand-type">jm<span>JM COMICS</span></span>
    </router-link>
    <div class="sidebar-navigation">
      <div class="nav-group">
        <span class="nav-heading">探索 / EXPLORE</span
        ><router-link
          v-for="item in explore"
          :key="item.path"
          :to="item.path"
          class="nav-item"
          :title="item.label"
          exact-active-class="active"
          ><AppIcon :name="item.icon" /><span>{{ item.label }}</span
          ><span class="nav-active-dot"
        /></router-link>
      </div>
      <div class="nav-group">
        <span class="nav-heading">我的 / COLLECTION</span
        ><router-link
          v-for="item in personal"
          :key="item.path"
          :to="item.path"
          class="nav-item"
          :title="item.label"
          active-class="active"
          ><AppIcon :name="item.icon" /><span>{{ item.label }}</span></router-link
        >
      </div>
    </div>
    <div class="sidebar-note">
      <AppIcon name="sparkles" :size="24" />
      <p>收藏好本，随时开看。</p>
      <span>在线浏览 · 收藏同步 · 本地下载</span>
      <div class="note-line" />
    </div>
    <div class="sidebar-bottom">
      <span v-if="punchNotice" class="punch-notice" role="status">{{ punchNotice }}</span
      ><button
        v-if="app.isLoggedIn"
        class="nav-item punch-btn"
        :disabled="punching"
        :title="'每日签到 · ' + (comics.daily?.eventName || 'JM')"
        @click="punchIn"
      >
        <AppIcon name="check" /><span>{{ punching ? '签到中…' : '每日签到' }}</span>
      </button>
      <router-link to="/settings" class="nav-item" active-class="active" title="偏好设置"
        ><AppIcon name="settings" /><span>偏好设置</span></router-link
      ><button
        class="nav-item collapse-btn"
        :title="app.sidebarCollapsed ? '展开导航' : '收起导航'"
        @click="app.toggleSidebar"
      >
        <AppIcon :name="app.sidebarCollapsed ? 'chevron-right' : 'chevron-left'" /><span
          >收起导航</span
        >
      </button>
    </div>
    <router-link to="/settings" class="sidebar-profile" title="账号信息"
      ><img
        v-if="comics.profile && comics.getAvatarUrl(comics.profile.avatar)"
        :src="comics.getAvatarUrl(comics.profile.avatar)"
        alt="头像"
        class="avatar" /><span v-else class="avatar"><AppIcon name="user" :size="19" /></span
      ><span class="profile-info"
        ><strong>{{ app.isLoggedIn ? comics.profile?.name || '漫画爱好者' : '游客模式' }}</strong
        ><small>{{
          app.isLoggedIn
            ? comics.profile
              ? 'Lv.' + comics.profile.level + ' · ' + comics.profile.coin + ' JCoin'
              : 'JM 账号'
            : '登录后可同步收藏与历史'
        }}</small></span
      ><button v-if="!app.isLoggedIn" class="text-btn" @click.prevent.stop="goLogin">登录</button
      ><AppIcon v-else name="chevron-right" :size="15"
    /></router-link>
  </aside>
</template>

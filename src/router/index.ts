import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'home',
    component: () => import('../views/HomeView.vue')
  },
  {
    path: '/categories',
    name: 'categories',
    component: () => import('../views/CategoriesView.vue')
  },
  {
    path: '/ranking',
    name: 'ranking',
    component: () => import('../views/RankingView.vue')
  },
  {
    path: '/weekly',
    name: 'weekly',
    component: () => import('../views/WeeklyView.vue')
  },
  {
    path: '/search',
    name: 'search',
    component: () => import('../views/SearchView.vue')
  },
  {
    path: '/comic/:id',
    name: 'comic-detail',
    component: () => import('../views/ComicDetailView.vue')
  },
  {
    path: '/reader/:comicId/:chapterId',
    name: 'reader',
    component: () => import('../views/ReaderView.vue')
  },
  {
    path: '/favorites',
    name: 'favorites',
    component: () => import('../views/FavoritesView.vue')
  },
  {
    path: '/history',
    name: 'history',
    component: () => import('../views/HistoryView.vue')
  },
  {
    path: '/downloads',
    name: 'downloads',
    component: () => import('../views/DownloadsView.vue')
  },
  {
    path: '/library',
    name: 'library',
    component: () => import('../views/LibraryView.vue')
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('../views/SettingsView.vue')
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router

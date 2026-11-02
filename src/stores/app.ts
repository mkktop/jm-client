import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'

function preference(key: string, fallback: string) {
  try {
    return localStorage.getItem(key) || fallback
  } catch {
    return fallback
  }
}
function savePreference(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    /* Preferences remain usable for this session. */
  }
}

const GUEST_KEY = 'jm-guest'

export const useAppStore = defineStore('app', {
  state: () => ({
    isLoggedIn: false,
    isGuest: false,
    isLoading: false,
    downloadDir: '',
    sidebarCollapsed: preference('jm-sidebar', 'false') === 'true',
    theme: preference('jm-theme', 'light') === 'dark' ? 'dark' : 'light',
    compactCards: preference('jm-compact', 'false') === 'true'
  }),
  getters: {
    canEnter(state) {
      return state.isLoggedIn || state.isGuest
    }
  },
  actions: {
    async checkLogin() {
      try {
        this.isLoggedIn = await invoke<boolean>('is_logged_in')
      } catch {
        this.isLoggedIn = false
      }
      if (!this.isLoggedIn) {
        this.isGuest = preference(GUEST_KEY, 'false') === 'true'
      }
      return this.isLoggedIn
    },
    async login(account: string, password: string) {
      this.isLoading = true
      try {
        await invoke('login', { req: { username: account, password } })
        this.isLoggedIn = true
        this.isGuest = false
        savePreference(GUEST_KEY, 'false')
        return true
      } finally {
        this.isLoading = false
      }
    },
    enterGuest() {
      this.isGuest = true
      savePreference(GUEST_KEY, 'true')
    },
    async logout() {
      try {
        await invoke('logout')
      } catch {
        // 即使后端清理失败，也清掉本地状态
      }
      this.isLoggedIn = false
      this.isGuest = false
      savePreference(GUEST_KEY, 'false')
    },
    async loadDownloadDir() {
      try {
        this.downloadDir = await invoke<string>('get_download_dir')
      } catch {
        this.downloadDir = ''
      }
    },
    toggleSidebar() {
      this.sidebarCollapsed = !this.sidebarCollapsed
      savePreference('jm-sidebar', String(this.sidebarCollapsed))
    },
    toggleTheme() {
      this.theme = this.theme === 'light' ? 'dark' : 'light'
      savePreference('jm-theme', this.theme)
    },
    toggleCompactCards() {
      this.compactCards = !this.compactCards
      savePreference('jm-compact', String(this.compactCards))
    }
  }
})

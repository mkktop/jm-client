<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '../stores/app'
import AppIcon from '../components/AppIcon.vue'
import ReadingArtwork from '../components/ReadingArtwork.vue'
import BrandMark from '../components/BrandMark.vue'
const router = useRouter()
const app = useAppStore()
const account = ref('')
const password = ref('')
const visible = ref(false)
const error = ref('')
async function handleLogin() {
  if (app.isLoading) return
  if (!account.value.trim() || !password.value) {
    error.value = '请输入 JM 用户名和密码'
    return
  }
  error.value = ''
  try {
    await app.login(account.value.trim(), password.value)
    password.value = ''
    router.push('/')
  } catch (e) {
    error.value = String(e)
  }
}
function browseAsGuest() {
  app.enterGuest()
  router.push('/')
}
</script>
<template>
  <div class="login-page">
    <section class="login-story">
      <div class="login-brand"><BrandMark /><span>jm</span></div>
      <div class="login-story-text">
        <div class="hero-kicker">JM · COMICS</div>
        <h1>你的私人<br />漫画库。</h1>
        <p>禁漫天堂桌面客户端</p>
      </div>
      <ReadingArtwork /><span class="login-story-footer"
        >BROWSE · COLLECT · DOWNLOAD <span>JM COMICS · 18+</span></span
      >
    </section>
    <section class="login-panel">
      <button
        class="icon-btn login-theme"
        :title="app.theme === 'light' ? '切换深色外观' : '切换浅色外观'"
        @click="app.toggleTheme"
      >
        <AppIcon :name="app.theme === 'light' ? 'moon' : 'sun'" />
      </button>
      <div class="login-form-wrap">
        <span class="eyebrow">SIGN IN TO JM</span>
        <h2>欢迎回来。</h2>
        <p class="login-intro">
          登录 JM 账号可同步收藏、评论、点赞、签到和观看历史；也可以直接游客浏览。
        </p>
        <form class="login-form" @submit.prevent="handleLogin">
          <div class="form-field">
            <label for="account">JM 用户名</label>
            <div class="field-with-icon">
              <AppIcon name="user" :size="17" /><input
                id="account"
                v-model="account"
                class="input"
                autocomplete="username"
                autocapitalize="off"
                :spellcheck="false"
                placeholder="输入 JM 用户名"
                :disabled="app.isLoading"
              />
            </div>
          </div>
          <div class="form-field">
            <label for="password">密码</label>
            <div class="field-with-icon">
              <AppIcon name="lock" :size="17" /><input
                id="password"
                v-model="password"
                class="input"
                :type="visible ? 'text' : 'password'"
                autocomplete="current-password"
                placeholder="输入你的密码"
                :disabled="app.isLoading"
              /><button
                type="button"
                class="icon-btn password-toggle"
                :title="visible ? '隐藏密码' : '显示密码'"
                :aria-pressed="visible"
                @click="visible = !visible"
              >
                <AppIcon name="eye" :size="17" />
              </button>
            </div>
          </div>
          <div v-if="error" role="alert" class="notice error">
            <AppIcon name="alert" :size="16" />{{ error }}
          </div>
          <button class="btn btn-primary login-submit" :disabled="app.isLoading">
            <span v-if="app.isLoading" class="spinner" />{{
              app.isLoading ? '正在登录…' : '登录 JM'
            }}<AppIcon v-if="!app.isLoading" name="arrow" :size="17" />
          </button>
          <button
            type="button"
            class="btn btn-secondary"
            style="width: 100%; margin-top: 10px"
            :disabled="app.isLoading"
            @click="browseAsGuest"
          >
            <AppIcon name="compass" :size="16" /> 以游客身份浏览
          </button>
        </form>
        <div class="login-help">
          <AppIcon name="lock" :size="13" /><span>登录状态会保存在本机；游客可浏览、阅读和下载。</span>
        </div>
      </div>
      <footer class="login-panel-footer">
        <span>JM DESKTOP</span><span>禁漫天堂桌面客户端</span>
      </footer>
    </section>
  </div>
</template>

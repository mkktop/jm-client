<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { invoke, isTauri } from '@tauri-apps/api/core'
import AppIcon from './AppIcon.vue'

interface UpdateCheck {
  status: 'available' | 'latest' | 'unpublished'
  currentVersion: string
  latestVersion: string | null
  releaseNotes: string | null
  publishedAt: string | null
}

defineProps<{ currentVersion: string }>()
const emit = defineEmits<{ version: [version: string] }>()
const desktop = isTauri()
const checking = ref(false)
const opening = ref(false)
const result = ref<UpdateCheck | null>(null)
const error = ref('')
const downloadMessage = ref('')
const statusMessage = computed(() => {
  if (result.value?.status === 'available') return `发现新版本 v${result.value.latestVersion}`
  if (result.value?.status === 'latest') return '当前已是最新版本'
  if (result.value?.status === 'unpublished') return '项目暂未发布正式版本，请稍后再检查。'
  return ''
})
const publishedDate = computed(() => {
  if (!result.value?.publishedAt) return ''
  const date = new Date(result.value.publishedAt)
  return Number.isNaN(date.getTime()) ? '' : date.toLocaleDateString('zh-CN')
})

onMounted(async () => {
  if (!desktop) return
  try {
    emit('version', await getVersion())
  } catch {
    // 手动检查仍可从后端获取实际版本。
  }
})

async function checkUpdates() {
  if (checking.value || opening.value || !desktop) return
  checking.value = true
  result.value = null
  error.value = ''
  downloadMessage.value = ''
  try {
    result.value = await invoke<UpdateCheck>('check_for_updates')
    emit('version', result.value.currentVersion)
  } catch (cause) {
    error.value = '检查失败：' + String(cause)
  } finally {
    checking.value = false
  }
}

async function downloadUpdate() {
  if (opening.value || checking.value || result.value?.status !== 'available') return
  opening.value = true
  error.value = ''
  downloadMessage.value = ''
  try {
    await invoke('open_update_download')
    downloadMessage.value = '已在浏览器打开新版下载页，下载后关闭客户端并运行安装包完成升级。'
  } catch (cause) {
    error.value = '打开下载页失败：' + String(cause)
  } finally {
    opening.value = false
  }
}
</script>

<template>
  <section
    class="setting-group app-update"
    aria-labelledby="app-update-title"
    :aria-busy="checking"
  >
    <div class="setting-title">
      <AppIcon name="refresh" />
      <div>
        <h2 id="app-update-title">版本与更新</h2>
        <p>检查 JM Client 的最新正式版本</p>
      </div>
    </div>
    <div class="setting-row">
      <div>
        <strong>当前版本</strong>
        <p>{{ currentVersion ? 'v' + currentVersion : '未知' }}</p>
      </div>
      <button
        class="btn btn-secondary btn-sm"
        :disabled="checking || opening || !desktop"
        @click="checkUpdates"
      >
        <span v-if="checking" class="spinner" />{{ checking ? '正在检查…' : '检查更新' }}
      </button>
    </div>
    <p v-if="!desktop" class="hint">请在桌面客户端中检查更新。</p>
    <div
      v-if="statusMessage || error || downloadMessage"
      class="update-feedback"
      aria-live="polite"
    >
      <p v-if="statusMessage" class="setting-message" role="status">{{ statusMessage }}</p>
      <p v-if="error" class="setting-message error" role="alert">{{ error }}</p>
      <p v-if="downloadMessage" class="setting-message" role="status">{{ downloadMessage }}</p>
    </div>
    <div v-if="result?.status === 'available'" class="update-release">
      <p v-if="publishedDate" class="hint">发布时间：{{ publishedDate }}</p>
      <div v-if="result.releaseNotes" class="update-notes">
        <h3>更新说明</h3>
        <p>{{ result.releaseNotes }}</p>
      </div>
      <div class="setting-actions">
        <button class="btn btn-primary btn-sm" :disabled="opening" @click="downloadUpdate">
          <AppIcon name="download" :size="14" />{{ opening ? '正在打开…' : '下载新版本' }}
        </button>
      </div>
      <p class="hint">在发布页选择适合系统的安装包，下载后关闭客户端并安装新版本。</p>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { Comic } from '../types'
import { useComicStore } from '../stores/comic'
import AppIcon from './AppIcon.vue'
const props = defineProps<{ comic: Comic; rank?: number }>()
const emit = defineEmits<{ click: [] }>()
const store = useComicStore()
const failed = ref(false)
const thumbUrl = computed(() => store.getThumbUrl(props.comic))
watch(thumbUrl, () => {
  failed.value = false
})
const count = (n: number) => (n >= 10000 ? (n / 10000).toFixed(1) + '万' : n.toLocaleString())
const tags = computed(() => (props.comic.categories || []).slice(0, 2))
const downloadState = computed<'local' | 'history' | undefined>(() => {
  if (store.localIds.has(props.comic._id)) return 'local'
  if (store.historyIds.has(props.comic._id)) return 'history'
  return undefined
})
</script>
<template>
  <button class="comic-card" @click="emit('click')" :aria-label="'查看 ' + comic.title">
    <span class="comic-cover"
      ><img
        v-if="thumbUrl && !failed"
        :src="thumbUrl"
        :alt="comic.title"
        loading="lazy"
        @error="failed = true" /><span v-else class="cover-fallback"
        ><AppIcon name="book" :size="36" /><span>JM LIBRARY</span></span
      ><span v-if="rank" class="rank-badge" :class="{ 'rank-top': rank <= 3 }">{{
        String(rank).padStart(2, '0')
      }}</span
      ><span class="cover-status">JM</span
      ><span class="cover-overlay"
        ><span>查看详情 <AppIcon name="arrow" :size="16" /></span></span
    ></span>
    <span class="comic-info"
      ><strong class="comic-title" :title="comic.title">{{ comic.title }}</strong
      ><span class="comic-author">{{ comic.author || '作者未署名' }}</span
      ><span v-if="tags.length || downloadState" class="comic-tags"
        ><span v-for="tag in tags" :key="tag" class="comic-tag">{{ tag }}</span
        ><span
          v-if="downloadState"
          class="comic-tag"
          :class="downloadState === 'local' ? 'is-downloaded' : 'is-history'"
          :title="downloadState === 'local' ? '本地已有下载' : '下载历史里有，本地文件已删'"
          >{{ downloadState === 'local' ? '已下载' : '曾下载' }}</span
        ></span
      ><span class="comic-meta"
        ><span
          >{{ comic.categories?.[0] || '漫画'
          }}<template v-if="comic.pagesCount"> · {{ comic.pagesCount }}P</template></span
        ><span v-if="comic.likesCount" class="comic-likes"
          ><AppIcon name="heart" :size="12" />{{ count(comic.likesCount) }}</span
        ></span
      ></span
    >
  </button>
</template>

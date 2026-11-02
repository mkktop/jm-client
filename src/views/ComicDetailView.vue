<script setup lang="ts">
import { onMounted, onUnmounted, computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../stores/app'
import { useComicStore } from '../stores/comic'
import ComicCard from '../components/ComicCard.vue'
import AppIcon from '../components/AppIcon.vue'
import EmptyState from '../components/EmptyState.vue'
import Paginator from '../components/Paginator.vue'
import { readingProgress } from '../utils/readerPreferences'
import type { CommentItem, Episode } from '../types'

const route = useRoute()
const router = useRouter()
const app = useAppStore()
const store = useComicStore()
const comicId = String(route.params.id)
const comic = computed(() => (store.currentComic?._id === comicId ? store.currentComic : null))
const tags = computed(() => comic.value?.tags || [])
const works = computed(() => comic.value?.works || [])
const actors = computed(() => comic.value?.actors || [])
const downloading = ref<string[]>([])
const queued = ref<string[]>([])
const downloadedChapters = ref(new Set<string>())
const historyChapters = ref(new Set<string>())
const revealed = ref(new Set<string>())
const acting = ref(false)
const commentLoading = ref(false)
const posting = ref(false)
const commentText = ref('')
const commentSpoiler = ref(false)
const notice = ref('')
const actionError = ref(false)
const failedCover = ref(false)
const commentsTop = ref<HTMLElement>()
const reversed = ref(false)
const episodes = computed(() => (reversed.value ? [...store.episodes].reverse() : store.episodes))
const saved = readingProgress(comicId)
const resumeEpisode = computed(
  () => saved && store.episodes.find((episode) => episode._id === saved.chapterId)
)
async function load() {
  await store.fetchComic(comicId)
}
onMounted(() => {
  store.comments = []
  load()
  store.fetchComments(comicId)
  refreshDownloaded()
  window.addEventListener('focus', refreshDownloaded)
})
onUnmounted(() => window.removeEventListener('focus', refreshDownloaded))
async function refreshDownloaded() {
  try {
    const ids = await invoke<string[]>('get_downloaded_chapter_ids', { comicId })
    downloadedChapters.value = new Set(ids)
  } catch {
    downloadedChapters.value = new Set()
  }
  try {
    const ids = await invoke<string[]>('get_history_chapter_ids', { comicId })
    historyChapters.value = new Set(ids)
  } catch {
    historyChapters.value = new Set()
  }
}
async function toggle(kind: 'favourite' | 'like') {
  if (!comic.value || acting.value) return
  acting.value = true
  notice.value = ''
  try {
    if (kind === 'favourite') await store.toggleFavourite(comicId)
    else await store.toggleLike(comicId)
    actionError.value = false
    notice.value = kind === 'favourite' ? '收藏状态已更新' : '点赞状态已更新'
  } catch (e) {
    actionError.value = true
    notice.value = '操作失败：' + e
  } finally {
    acting.value = false
  }
}
async function download(ep: Episode) {
  if (
    !comic.value ||
    downloading.value.includes(ep._id) ||
    queued.value.includes(ep._id)
  )
    return
  downloading.value.push(ep._id)
  notice.value = ''
  try {
    await invoke('start_download', {
      req: {
        comic_id: comicId,
        comic_title: comic.value.title,
        chapter_id: ep._id,
        chapter_title: ep.title,
        episode_order: ep.order
      }
    })
    queued.value.push(ep._id)
    actionError.value = false
    notice.value = '「' + ep.title + '」已加入下载队列'
  } catch (e) {
    actionError.value = true
    notice.value = '下载失败：' + e
  } finally {
    downloading.value = downloading.value.filter((id) => id !== ep._id)
  }
}
const pendingEpisodes = computed(() =>
  store.episodes.filter(
    (episode) =>
      !downloadedChapters.value.has(episode._id) &&
      !downloading.value.includes(episode._id) &&
      !queued.value.includes(episode._id)
  )
)
async function downloadAll() {
  if (!comic.value || acting.value) return
  const targets = pendingEpisodes.value
  if (!targets.length) return
  notice.value = ''
  let ok = 0
  for (const episode of targets) {
    try {
      await invoke('start_download', {
        req: {
          comic_id: comicId,
          comic_title: comic.value.title,
          chapter_id: episode._id,
          chapter_title: episode.title,
          episode_order: episode.order
        }
      })
      queued.value.push(episode._id)
      ok++
    } catch {
      // 单章入队失败不阻塞其余章节
    }
  }
  if (ok) {
    actionError.value = false
    notice.value = `已把 ${ok} 个章节加入下载队列`
  } else {
    actionError.value = true
    notice.value = '批量下载失败，请稍后重试'
  }
}
function read(ep: Episode) {
  router.push('/reader/' + comicId + '/' + ep._id)
}
function goBack() {
  const previous = router.options.history.state.back
  if (typeof previous === 'string') {
    const target = router.resolve(previous)
    if (target.matched.length && target.name !== 'reader') {
      router.back()
      return
    }
  }
  router.replace({ name: 'home' })
}
async function goToCommentPage(page: number) {
  if (commentLoading.value) return
  commentLoading.value = true
  commentsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  await store.fetchComments(comicId, page)
  commentLoading.value = false
}
function reveal(comment: CommentItem) {
  const next = new Set(revealed.value)
  next.add(comment._id)
  revealed.value = next
}
function cleanComment(text: string): string {
  return text
    .replace(/<[^>]*>/g, '')
    .replace(/&nbsp;/g, ' ')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .trim()
}
async function submitComment() {
  if (!app.isLoggedIn) {
    actionError.value = true
    notice.value = '请先登录后再评论'
    return
  }
  if (!commentText.value.trim() || posting.value) return
  posting.value = true
  notice.value = ''
  try {
    const result = await store.postComment(
      comicId,
      commentText.value.trim(),
      undefined,
      commentSpoiler.value
    )
    commentText.value = ''
    commentSpoiler.value = false
    actionError.value = false
    notice.value = result.message || '评论已发布'
  } catch (e) {
    actionError.value = true
    notice.value = '评论失败：' + e
  } finally {
    posting.value = false
  }
}
const count = (n = 0) => (n >= 10000 ? (n / 10000).toFixed(1) + '万' : n.toLocaleString())
</script>
<template>
  <div class="page comic-detail">
    <button class="text-btn detail-back" @click="goBack">
      <AppIcon name="back" :size="16" />返回上一页
    </button>
    <div v-if="store.loading && !comic" class="loading">
      <span class="spinner" />正在加载漫画详情…
    </div>
    <EmptyState
      v-else-if="!comic"
      error
      title="这部作品暂时没有打开"
      :description="store.error || '请稍后重新加载。'"
      ><button class="btn btn-primary" @click="load">重新加载</button></EmptyState
    >
    <template v-else
      ><section class="detail-header">
        <div class="cover-large">
          <img
            v-if="store.getThumbUrl(comic) && !failedCover"
            :src="store.getThumbUrl(comic)"
            :alt="comic.title"
            @error="failedCover = true"
          /><span v-else class="cover-fallback"
            ><AppIcon name="book" :size="42" /><span>JM LIBRARY</span></span
          >
        </div>
        <div class="detail-info">
          <div class="eyebrow">JM COMIC · {{ comic.categories[0] || '漫画' }}</div>
          <h1>{{ comic.title }}</h1>
          <button
            v-if="comic.author"
            class="detail-author author-link"
            :title="'搜索作者「' + comic.author + '」'"
            @click="router.push({ name: 'search', query: { q: comic.author } })"
          >
            {{ comic.author }}
          </button>
          <p v-else class="detail-author">作者未署名</p>
          <p
            v-if="comic.created_at || comic.updated_at"
            class="detail-dates"
            :title="'发布 ' + (comic.created_at || '未知') + ' · 更新 ' + (comic.updated_at || '未知')"
          >
            <template v-if="comic.created_at">发布 {{ comic.created_at.slice(0, 10) }}</template
            ><template
              v-if="
                comic.updated_at &&
                comic.updated_at.slice(0, 10) !== (comic.created_at || '').slice(0, 10)
              "
              >　更新 {{ comic.updated_at.slice(0, 10) }}</template
            >
          </p>
          <div class="detail-stats">
            <div>
              <strong>{{ count(comic.pagesCount) }}</strong
              ><span>页</span>
            </div>
            <div>
              <strong>{{ count(comic.epsCount) }}</strong
              ><span>章</span>
            </div>
            <div>
              <strong>{{ count(comic.likesCount) }}</strong
              ><span>喜欢</span>
            </div>
            <div>
              <strong>{{ count(comic.viewsCount) }}</strong
              ><span>浏览</span>
            </div>
            <div>
              <strong>{{ count(comic.commentCount) }}</strong
              ><span>评论</span>
            </div>
          </div>
          <div v-if="tags.length" class="categories detail-tags">
            <button
              v-for="tag in tags"
              :key="tag"
              class="tag tag-chip"
              style="border: 0"
              :title="'按标签「' + tag + '」搜索'"
              @click="router.push({ name: 'search', query: { q: tag, type: 'tag' } })"
            >
              # {{ tag }}
            </button>
          </div>
          <div v-if="works.length" class="categories detail-tags">
            <span class="detail-meta-label">作品</span>
            <button
              v-for="work in works"
              :key="work"
              class="tag"
              style="border: 0"
              @click="router.push({ name: 'search', query: { q: work } })"
            >
              {{ work }}
            </button>
          </div>
          <div v-if="actors.length" class="categories detail-tags">
            <span class="detail-meta-label">角色</span>
            <button
              v-for="actor in actors"
              :key="actor"
              class="tag"
              style="border: 0"
              @click="router.push({ name: 'search', query: { q: actor } })"
            >
              {{ actor }}
            </button>
          </div>
          <p v-if="comic.description" class="description">{{ comic.description }}</p>
          <div class="detail-actions">
            <button
              v-if="store.episodes[0]"
              class="btn btn-primary"
              @click="read(resumeEpisode || store.episodes[0])"
            >
              <AppIcon name="book" :size="16" />{{
                resumeEpisode ? '继续阅读 · 第 ' + resumeEpisode.order + ' 话' : '开始阅读'
              }}<AppIcon name="arrow" :size="15" /></button
            ><button
              class="btn btn-secondary"
              :class="{ selected: comic.isFavourite }"
              :disabled="acting"
              @click="toggle('favourite')"
            >
              <AppIcon name="heart" :size="16" />{{
                comic.isFavourite ? '已加入收藏' : '加入收藏'
              }}</button
            ><button
              class="btn btn-secondary"
              :class="{ selected: comic.isLiked }"
              :disabled="acting"
              @click="toggle('like')"
            >
              <AppIcon name="sparkles" :size="16" />{{ comic.isLiked ? '已喜欢' : '喜欢' }}
            </button>
          </div>
        </div>
      </section>
      <div
        v-if="notice"
        role="status"
        class="notice"
        :class="{ error: actionError }"
        style="margin-top: 17px"
      >
        <AppIcon :name="actionError ? 'alert' : 'check'" :size="15" />{{ notice
        }}<router-link v-if="!actionError" to="/downloads" class="text-btn">查看下载</router-link>
      </div>
      <section class="detail-section">
        <div class="section-heading">
          <h2>
            章节目录<small>{{ store.episodes.length }} CHAPTERS</small>
          </h2>
          <div class="section-controls">
            <button
              class="btn btn-secondary btn-sm"
              :disabled="!pendingEpisodes.length"
              :title="pendingEpisodes.length ? '下载全部未下载章节' : '所有章节都已下载或在队列中'"
              @click="downloadAll"
            >
              <AppIcon name="download" :size="14" />下载整本<template v-if="pendingEpisodes.length"
                >（{{ pendingEpisodes.length }} 话）</template
              >
            </button><button class="text-btn" @click="reversed = !reversed">
              <AppIcon name="list" :size="14" />{{ reversed ? '倒序排列' : '正序排列' }}
            </button>
          </div>
        </div>
        <EmptyState v-if="!episodes.length" title="暂无章节" description="稍后再来看看。" />
        <div v-else class="episodes-grid">
          <article v-for="ep in episodes" :key="ep._id" class="episode-card">
            <div class="ep-info">
              <span class="ep-order">{{ String(ep.order).padStart(2, '0') }}</span
              ><span class="ep-title" :title="ep.title">{{ ep.title }}</span
              ><span v-if="downloadedChapters.has(ep._id)" class="ep-badge">已下载</span
              ><span
                v-else-if="historyChapters.has(ep._id)"
                class="ep-badge muted"
                title="下载历史里有这一章，但本地文件已不在"
                >曾下载</span
              >
            </div>
            <div class="ep-actions">
              <button class="btn btn-secondary btn-sm" @click="read(ep)">阅读</button
              ><button
                class="icon-btn"
                :title="
                  downloadedChapters.has(ep._id)
                    ? '已下载到本地'
                    : queued.includes(ep._id)
                      ? '已加入队列'
                      : '下载 ' + ep.title
                "
                :disabled="
                  downloading.includes(ep._id) ||
                  queued.includes(ep._id) ||
                  downloadedChapters.has(ep._id)
                "
                @click="download(ep)"
              >
                <AppIcon
                  :name="
                    downloadedChapters.has(ep._id) || queued.includes(ep._id)
                      ? 'check'
                      : 'download'
                  "
                  :size="16"
                />
              </button>
            </div>
          </article>
        </div>
      </section>
      <section v-if="store.related.length" class="detail-section">
        <div class="section-heading">
          <h2>相关推荐<small>RELATED ALBUMS</small></h2>
        </div>
        <div class="comic-grid">
          <ComicCard
            v-for="item in store.related"
            :key="item._id"
            :comic="item"
            @click="router.push('/comic/' + item._id)"
          />
        </div>
      </section>
      <section ref="commentsTop" class="detail-section">
        <div class="section-heading">
          <h2>
            评论区<small
              >{{ store.commentPages > 1
                ? `第 ${store.commentPage} / ${store.commentPages} 页`
                : `${store.commentTotal} COMMENTS` }}</small
            >
          </h2>
        </div>
        <div class="comment-form">
          <textarea
            v-model="commentText"
            class="input comment-input"
            rows="3"
            :placeholder="app.isLoggedIn ? '说点什么…' : '登录后可发表评论'"
            :disabled="!app.isLoggedIn || posting"
          />
          <div class="comment-form-actions">
            <label class="comment-spoiler">
              <input v-model="commentSpoiler" type="checkbox" :disabled="!app.isLoggedIn" />
              含剧透
            </label>
            <button
              class="btn btn-primary btn-sm"
              :disabled="!app.isLoggedIn || posting || !commentText.trim()"
              @click="submitComment"
            >
              <span v-if="posting" class="spinner" />发表评论
            </button>
          </div>
        </div>
        <div v-if="store.comments.length" class="comments-list">
          <article v-for="c in store.comments" :key="c._id" class="comment-item">
            <img
              v-if="store.getAvatarUrl(c.user?.avatar)"
              class="comment-avatar"
              :src="store.getAvatarUrl(c.user?.avatar)"
              alt="读者头像"
            /><span v-else class="comment-avatar"><AppIcon name="user" :size="16" /></span>
            <div class="comment-body">
              <div class="comment-head">
                <strong>{{ c.user?.name || '匿名读者' }}</strong
                ><span class="comment-level">Lv.{{ c.user?.level || 0 }}</span
                ><time class="comment-time">{{ c.createdAt?.slice(0, 20) }}</time>
              </div>
              <template v-if="c.isSpoiler && !revealed.has(c._id)">
                <button class="spoiler-mask" @click="reveal(c)">
                  <AppIcon name="alert" :size="15" /> 剧透内容，点击查看
                </button>
              </template>
              <p v-else class="comment-content">{{ cleanComment(c.content) }}</p>
              <div class="comment-foot">
                <AppIcon name="heart" :size="12" />{{ c.likesCount || 0 }}
              </div>
            </div>
          </article>
        </div>
        <EmptyState v-else title="暂无评论" description="暂时没有可展示的评论。" icon="message" />
        <Paginator
          :page="store.commentPage"
          :pages="store.commentPages"
          :loading="commentLoading"
          @change="goToCommentPage"
        />
      </section>
    </template>
  </div>
</template>
<style scoped>
.detail-meta-label {
  font-size: 10px;
  color: var(--text-muted);
  align-self: center;
  margin-right: 4px;
}
.comment-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 18px;
}
.comment-input {
  width: 100%;
  resize: vertical;
  min-height: 64px;
}
.comment-form-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.comment-spoiler {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-secondary);
}
.spoiler-mask {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border: 1px dashed var(--border);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-secondary);
  font-size: 12px;
}
</style>

import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type {
  ActionResult,
  CategoriesResult,
  Category,
  Comic,
  CommentItem,
  DailyInfo,
  Episode,
  FavoriteFolder,
  FavoriteResult,
  ForumPage,
  Picture,
  PicturePage,
  ProfileUser,
  PromoteSection,
  SearchResult,
  WatchListResult,
  WeekList,
  WeeklyInfo
} from '../types'
import { protocolUrl, toDisplayUrl } from '../utils/imageUrl'
import { readPreference, writePreference } from '../utils/readerPreferences'

const BLOCKED_TAGS_KEY = 'jm-blocked-tags'
const BLOCKED_SCOPES_KEY = 'jm-blocked-tag-scopes'

export type BlockedTagScope = 'search' | 'home' | 'categories' | 'weekly'

function pagesFor(total: number, pageSize: number): number {
  if (total <= pageSize) return 1
  return Math.ceil(total / pageSize)
}

export const useComicStore = defineStore('comic', {
  state: () => ({
    // 发现
    promote: [] as PromoteSection[],
    ranking: [] as Comic[],
    rankingPeriod: 'mv_t',
    latest: [] as Comic[],
    latestPage: 1,
    latestPages: 1,
    // 每周必看
    weeklyInfo: null as WeeklyInfo | null,
    weeklyList: [] as Comic[],
    weeklyCategoryId: '',
    weeklyTypeId: 'manga',
    weeklyPage: 1,
    weeklyPages: 1,
    // 分类
    categories: [] as Category[],
    categoriesLoading: false,
    categoryComics: [] as Comic[],
    categoryPage: 1,
    categoryPages: 1,
    // 搜索
    searchResults: [] as Comic[],
    searchPage: 1,
    searchPages: 1,
    searchTotal: 0,
    searchType: 'site',
    // 详情
    currentComic: null as Comic | null,
    episodes: [] as Episode[],
    related: [] as Comic[],
    // 阅读
    pictures: [] as Picture[],
    pictureTotal: 0,
    picturePages: 0,
    pictureCursor: 0,
    fetchingPictures: false,
    picturesError: null as string | null,
    picturesLocal: false,
    // 评论
    comments: [] as CommentItem[],
    commentPage: 1,
    commentPages: 1,
    commentTotal: 0,
    // 收藏
    favourites: [] as Comic[],
    favouriteFolders: [] as FavoriteFolder[],
    favouriteFolderId: '0',
    favouriteSort: 'mr',
    favouritePage: 1,
    favouritePages: 1,
    favouriteTotal: 0,
    // 观看历史
    watchList: [] as Comic[],
    watchPage: 1,
    watchPages: 1,
    watchTotal: 0,
    // 账号
    profile: null as ProfileUser | null,
    daily: null as DailyInfo | null,
    // 本地状态
    historyIds: new Set<string>(),
    localIds: new Set<string>(),
    downloadStatesFetchedAt: 0,
    blockedTags: readPreference<string[]>(BLOCKED_TAGS_KEY, []),
    blockedTagScopes: {
      search: true,
      home: false,
      categories: false,
      weekly: false,
      ...readPreference<Partial<Record<BlockedTagScope, boolean>>>(BLOCKED_SCOPES_KEY, {})
    } as Record<BlockedTagScope, boolean>,
    loading: false,
    error: null as string | null,
    requestId: 0,
    commentRequestId: 0,
    // 首页各区块独立加载，互不覆盖请求；全局 loading 仍用于单资源页面。
    keyRequests: {} as Record<string, number>,
    loadingKeys: {} as Record<string, boolean>,
    errorKeys: {} as Record<string, string | null>
  }),
  actions: {
    cancelRequests() {
      this.requestId++
      this.commentRequestId++
      for (const key of Object.keys(this.keyRequests)) {
        this.keyRequests[key] = (this.keyRequests[key] || 0) + 1
      }
      this.loading = false
      this.error = null
    },
    async loadResourceKeyed<T>(
      key: string,
      request: () => Promise<T>,
      apply: (data: T) => void
    ) {
      const id = (this.keyRequests[key] || 0) + 1
      this.keyRequests[key] = id
      this.loadingKeys[key] = true
      this.errorKeys[key] = null
      try {
        const data = await request()
        if (id === this.keyRequests[key]) apply(data)
      } catch (e) {
        if (id === this.keyRequests[key]) this.errorKeys[key] = String(e)
      } finally {
        if (id === this.keyRequests[key]) this.loadingKeys[key] = false
      }
    },
    async loadResource<T>(request: () => Promise<T>, apply: (data: T) => void) {
      const id = ++this.requestId
      this.loading = true
      this.error = null
      try {
        const data = await request()
        if (id === this.requestId) apply(data)
      } catch (e) {
        if (id === this.requestId) this.error = String(e)
      } finally {
        if (id === this.requestId) this.loading = false
      }
    },
    filterBlocked(docs: Comic[], scope: BlockedTagScope): Comic[] {
      if (!this.blockedTags.length || !this.blockedTagScopes[scope]) return docs
      const blocked = new Set(this.blockedTags)
      return docs.filter((comic) => !(comic.tags || []).some((tag) => blocked.has(tag)))
    },
    blockTag(tag: string) {
      const value = tag.trim()
      if (!value || this.blockedTags.includes(value)) return
      this.blockedTags = [...this.blockedTags, value].slice(0, 30)
      writePreference(BLOCKED_TAGS_KEY, this.blockedTags)
    },
    unblockTag(tag: string) {
      this.blockedTags = this.blockedTags.filter((item) => item !== tag)
      writePreference(BLOCKED_TAGS_KEY, this.blockedTags)
    },
    setBlockedTagScope(scope: BlockedTagScope, enabled: boolean) {
      this.blockedTagScopes = { ...this.blockedTagScopes, [scope]: enabled }
      writePreference(BLOCKED_SCOPES_KEY, this.blockedTagScopes)
    },
    async fetchPromote() {
      await this.loadResourceKeyed(
        'promote',
        () => invoke<PromoteSection[]>('get_promote'),
        (data) => {
          this.promote = data
        }
      )
    },
    async fetchRanking(period?: string, category = '') {
      const targetPeriod = period || this.rankingPeriod
      this.rankingPeriod = targetPeriod
      await this.loadResourceKeyed(
        'ranking',
        () => invoke<SearchResult>('get_ranking', { page: 1, period: targetPeriod, category }),
        (data) => {
          this.ranking = this.filterBlocked(data.content, 'home')
        }
      )
    },
    async fetchLatest(page = 1) {
      await this.loadResourceKeyed(
        'latest',
        () => invoke<SearchResult>('get_latest_comics', { page }),
        (data) => {
          this.latest = this.filterBlocked(data.content, 'home')
          this.latestPage = page
          this.latestPages = pagesFor(data.total, 80)
        }
      )
    },
    async fetchWeeklyInfo() {
      await this.loadResourceKeyed(
        'weeklyInfo',
        () => invoke<WeeklyInfo>('get_weekly_info'),
        (data) => {
          this.weeklyInfo = data
          if (!this.weeklyCategoryId && data.categories.length) {
            this.weeklyCategoryId = data.categories[0].id
          }
          if (!data.types.some((item) => item.id === this.weeklyTypeId)) {
            this.weeklyTypeId = data.types[0]?.id || 'manga'
          }
        }
      )
    },
    async fetchWeekly(page = 1, categoryId?: string, typeId?: string) {
      this.weeklyCategoryId = categoryId ?? this.weeklyCategoryId
      this.weeklyTypeId = typeId ?? this.weeklyTypeId
      if (!this.weeklyCategoryId) await this.fetchWeeklyInfo()
      if (!this.weeklyCategoryId) return
      await this.loadResourceKeyed(
        'weekly',
        () =>
          invoke<WeekList>('get_weekly', {
            page,
            categoryId: this.weeklyCategoryId,
            typeId: this.weeklyTypeId
          }),
        (data) => {
          this.weeklyList = this.filterBlocked(data.list, 'weekly')
          this.weeklyPage = page
          this.weeklyPages = pagesFor(data.total, 20)
        }
      )
    },
    async fetchCategories() {
      // 分类目录与分类漫画是两个独立请求，不能共用 loadResource 的 requestId，
      // 否则深链进入 /categories?category=... 时分类结果会被目录请求丢弃。
      this.categoriesLoading = true
      this.error = null
      try {
        const data = await invoke<CategoriesResult>('get_categories')
        this.categories = data.categories
      } catch (e) {
        this.error = String(e)
      } finally {
        this.categoriesLoading = false
      }
    },
    async fetchCategoryComics(
      page: number,
      category: string,
      order = 'mr',
      time = 'a'
    ) {
      await this.loadResource(
        () =>
          invoke<SearchResult>('get_category_comics', {
            page,
            category,
            order,
            time
          }),
        (data) => {
          this.categoryComics = this.filterBlocked(data.content, 'categories')
          this.categoryPage = page
          this.categoryPages = pagesFor(data.total, 80)
        }
      )
    },
    async search(keyword: string, page: number, sort: string, mainTag: number, time: string) {
      let effective = keyword.trim()
      if (
        this.blockedTagScopes.search &&
        this.blockedTags.length &&
        effective &&
        !effective.startsWith('-')
      ) {
        effective += this.blockedTags.map((tag) => ` -${tag}`).join('')
      }
      await this.loadResource(
        () =>
          invoke<SearchResult>('search_comics', {
            req: {
              keyword: effective,
              page,
              sort,
              main_tag: mainTag,
              time
            }
          }),
        (data) => {
          const redirect = data.redirectAlbum
          this.searchResults = redirect
            ? [redirect]
            : this.filterBlocked(data.content, 'search')
          this.searchPage = page
          this.searchTotal = redirect ? 1 : data.total
          this.searchPages = redirect ? 1 : pagesFor(data.total, 80)
          this.searchType = data.searchType
        }
      )
    },
    async fetchComic(id: string) {
      if (this.currentComic?._id !== id) {
        this.currentComic = null
        this.episodes = []
        this.related = []
        this.comments = []
        this.commentPage = 1
        this.commentPages = 1
      }
      await this.loadResource(
        () => invoke<{ comic: Comic; episodes: Episode[] }>('get_comic_bundle', { comicId: id }),
        (data) => {
          this.currentComic = data.comic
          this.episodes = data.episodes
          this.related = data.comic.relatedList || []
        }
      )
    },
    async fetchPictures(comicId: string, chapterId: string) {
      const id = ++this.requestId
      this.pictures = []
      this.pictureTotal = 0
      this.picturePages = 0
      this.pictureCursor = 0
      this.fetchingPictures = false
      this.picturesError = null
      this.picturesLocal = false
      this.loading = true
      this.error = null
      try {
        const local = await invoke<string[] | null>('get_local_pictures', {
          comicId,
          chapterId
        })
        if (id !== this.requestId) return
        if (local && local.length) {
          this.pictures = local.map((path) => ({
            _id: path,
            filename: path,
            url: '',
            localPath: path
          }))
          this.pictureTotal = local.length
          this.picturePages = 1
          this.pictureCursor = 1
          this.picturesLocal = true
          this.loading = false
          return
        }
        const page = await invoke<PicturePage>('get_pictures_page', {
          comicId,
          chapterId,
          page: 1
        })
        if (id !== this.requestId) return
        this.pictures = page.docs
        this.pictureTotal = page.total || page.docs.length
        this.picturePages = Math.max(1, page.pages || 1)
        this.pictureCursor = 1
        this.loading = false
      } catch (e) {
        if (id === this.requestId) {
          this.error = String(e)
          this.loading = false
        }
      }
    },
    async fetchComments(comicId: string, page = 1) {
      const id = ++this.commentRequestId
      try {
        const data = await invoke<ForumPage>('get_forum', { comicId, page })
        if (id !== this.commentRequestId) return
        this.comments = data.docs
        this.commentPage = page
        this.commentPages = data.pages
        this.commentTotal = data.total
      } catch {
        if (id === this.commentRequestId && page === 1) {
          this.comments = []
          this.commentPage = 1
          this.commentPages = 1
          this.commentTotal = 0
        }
      }
    },
    async postComment(
      comicId: string,
      content: string,
      commentId?: string,
      spoiler = false
    ): Promise<ActionResult> {
      const result = await invoke<ActionResult>('post_comment', {
        req: { comic_id: comicId, content, comment_id: commentId, spoiler }
      })
      await this.fetchComments(comicId)
      return result
    },
    async toggleLike(comicId: string) {
      await invoke<ActionResult>('toggle_like', { comicId })
      await this.fetchComic(comicId)
    },
    async toggleFavourite(comicId: string) {
      await invoke<ActionResult>('toggle_favourite', { comicId })
      await this.fetchComic(comicId)
    },
    async fetchFavourites(page = 1, sort?: string, folderId?: string) {
      const targetSort = sort || this.favouriteSort
      const targetFolder = folderId ?? this.favouriteFolderId
      this.favouriteSort = targetSort
      this.favouriteFolderId = targetFolder
      await this.loadResource(
        () =>
          invoke<FavoriteResult>('get_my_favourites', {
            page,
            sort: targetSort,
            folderId: targetFolder
          }),
        (data) => {
          this.favourites = data.list
          this.favouriteFolders = data.folders
          this.favouritePage = page
          this.favouriteTotal = data.total
          this.favouritePages = pagesFor(data.total, 20)
        }
      )
    },
    async moveFavourite(comicId: string, folderId: string) {
      await invoke<ActionResult>('move_favourite', { comicId, folderId })
      await this.fetchFavourites(this.favouritePage, this.favouriteSort, this.favouriteFolderId)
    },
    async fetchWatchList(page = 1) {
      await this.loadResource(
        () => invoke<WatchListResult>('get_watch_list', { page }),
        (data) => {
          this.watchList = data.list
          this.watchPage = page
          this.watchTotal = data.total
          this.watchPages = pagesFor(data.total, 20)
        }
      )
    },
    async fetchProfile() {
      try {
        this.profile = await invoke<ProfileUser>('get_my_profile')
      } catch {
        this.profile = null
      }
    },
    async fetchDaily() {
      try {
        this.daily = await invoke<DailyInfo>('get_daily')
      } catch {
        this.daily = null
      }
    },
    async checkin(): Promise<ActionResult> {
      const result = await invoke<ActionResult>('daily_checkin')
      await this.fetchDaily()
      await this.fetchProfile()
      return result
    },
    async fetchDownloadStates(force = false) {
      if (!force && Date.now() - this.downloadStatesFetchedAt < 15000) return
      this.downloadStatesFetchedAt = Date.now()
      try {
        const sets = await invoke<{ history: string[]; local: string[] }>(
          'get_download_state_sets'
        )
        this.historyIds = new Set(sets.history)
        this.localIds = new Set(sets.local)
      } catch {
        // 忽略卡片徽标状态失败
      }
    },
    getThumbUrl(comic: Comic): string {
      if (!comic.thumb) return ''
      const path = comic.thumb.path
      const url =
        path.startsWith('http://') || path.startsWith('https://')
          ? path
          : comic.thumb.fileServer.replace(/\/$/, '') + '/static/' + path
      return protocolUrl('remote', url)
    },
    getAvatarUrl(avatar?: { path: string; fileServer: string }): string {
      if (!avatar) return ''
      const url =
        avatar.path.startsWith('http://') || avatar.path.startsWith('https://')
          ? avatar.path
          : avatar.fileServer.replace(/\/$/, '') + '/static/' + avatar.path
      return protocolUrl('remote', url)
    },
    getImageUrl(pic: Picture): string {
      return toDisplayUrl(pic.url, pic.localPath)
    }
  }
})

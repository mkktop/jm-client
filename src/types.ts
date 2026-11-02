export interface Thumb {
  path: string
  fileServer: string
}

export interface Comic {
  _id: string
  title: string
  author: string
  pagesCount: number
  epsCount: number
  categories: string[]
  tags: string[]
  works: string[]
  actors: string[]
  description: string
  thumb?: Thumb
  likesCount: number
  viewsCount: number
  commentCount: number
  isFavourite: boolean
  isLiked: boolean
  updated_at: string
  created_at: string
  relatedList: Comic[]
  price: string
  purchased: string
}

export interface Episode {
  _id: string
  title: string
  order: number
  updated_at?: string
}

export interface Picture {
  _id: string
  filename: string
  url: string
  localPath?: string
}

export interface PicturePage {
  docs: Picture[]
  total: number
  pages: number
}

export interface CommentUser {
  _id: string
  name: string
  level: number
  avatar?: Thumb
}

export interface CommentItem {
  _id: string
  content: string
  likesCount: number
  isSpoiler: boolean
  createdAt: string
  user: CommentUser
  replies: CommentItem[]
}

export interface ForumPage {
  docs: CommentItem[]
  total: number
  pages: number
}

export interface ProfileUser {
  _id: string
  name: string
  email?: string
  level: number
  levelName: string
  exp: number
  expPercent: number
  coin: number
  avatar?: Thumb
  favouriteCount: number
  maxFavouriteCount: number
  isPunched: boolean
}

export interface SubCategory {
  _id: string
  title: string
  slug: string
}

export interface Category {
  _id: string
  title: string
  slug: string
  totalAlbums: number
  subCategories: SubCategory[]
}

export interface CategoriesResult {
  categories: Category[]
  blocks: unknown
}

export interface WeekCategory {
  id: string
  title: string
  time: string
}

export interface WeekType {
  id: string
  title: string
}

export interface WeeklyInfo {
  categories: WeekCategory[]
  types: WeekType[]
}

export interface WeekList {
  total: number
  list: Comic[]
}

export interface SearchResult {
  searchQuery: string
  searchType: string
  total: number
  content: Comic[]
  redirectAlbum?: Comic
}

export interface FavoriteFolder {
  _id: string
  name: string
  ownerUserId: string
}

export interface FavoriteResult {
  list: Comic[]
  folders: FavoriteFolder[]
  total: number
  count: number
}

export interface PromoteSection {
  id: string
  title: string
  slug: string
  sectionType: string
  filterValue: string
  content: Comic[]
}

export interface PromoteList {
  total: number
  list: Comic[]
}

export interface WatchListResult {
  total: number
  list: Comic[]
}

export interface DailyRecord {
  date: string
  signed: boolean | null
  bonus: boolean
}

export interface DailyInfo {
  dailyId: string
  eventName: string
  currentProgress: string
  threeDaysCoin: string
  threeDaysExp: string
  sevenDaysCoin: string
  sevenDaysExp: string
  records: DailyRecord[]
}

export interface ActionResult {
  status: string
  message: string
  actionType: string
}

export interface DownloadTask {
  id: string
  comic_id: string
  comic_title: string
  chapter_id: string
  episode_order: number
  episode_title: string
  status: string
  total_images: number
  downloaded_images: number
  save_path: string
  created_at: string
}

export interface DownloadHistoryEntry {
  comic_id: string
  title: string
  chapter_ids: string[]
  first_downloaded_at?: string
  last_downloaded_at?: string
}

export interface LocalComic {
  comic_id: string
  title: string
  author?: string
  pages_count?: number
  category?: string
  eps_count?: number
  update_time?: string
  downloaded_episodes?: string[]
  cover_path?: string
}

export interface DownloadSettings {
  format: string
  concurrency: number
  interval_ms: number
}

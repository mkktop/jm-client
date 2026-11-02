import { test, expect, type Page } from '@playwright/test'

type Call = { cmd: string; args: any }

async function fixture(page: Page, loggedIn = true) {
  await page.route('http://jmimg.localhost/**', (route) => {
    const url = route.request().url()
    const n = Math.abs(url.split('').reduce((sum, ch) => sum + ch.charCodeAt(0), 0)) % 6
    const colors = ['#a3b79d', '#dbb18e', '#8ea7a0', '#b5a68c', '#a6ad87', '#cf9e8d']
    return route.fulfill({
      contentType: 'image/svg+xml',
      body:
        '<svg xmlns="http://www.w3.org/2000/svg" width="600" height="800"><rect width="600" height="800" fill="' +
        colors[n] +
        '"/><circle cx="400" cy="230" r="140" fill="#f3e6ca"/><path d="M0 590Q130 260 390 610T700 500V800H0" fill="#405e50"/><text x="45" y="105" font-family="Georgia" font-size="46" fill="#fff9ec">JM · TEST</text></svg>'
    })
  })
  await page.addInitScript(
    ({ loggedIn }) => {
      const comics = Array.from({ length: 6 }, (_, i) => ({
        _id: 'c' + i,
        title:
          ['雨后的书店', '山海之间', '漫长的午后', '森林来信', '直到星星亮起', '风经过的地方'][i] +
          (i > 2 ? ' · 续篇' : ''),
        author: ['青木', '林间', '白川'][i % 3],
        pagesCount: 36 + i * 8,
        epsCount: 2,
        categories: ['同人', '短篇'],
        tags: ['全彩', '純愛'],
        works: ['原创'],
        actors: ['角色' + i],
        description: '在生活的留白里，遇见那些微小而明亮的瞬间。',
        thumb: { path: 'http://jm.test/cover/' + i, fileServer: '' },
        likesCount: 1250 + i * 532,
        viewsCount: 9000 + i * 1000,
        commentCount: 3,
        isFavourite: false,
        isLiked: false,
        updated_at: '2026-11-01',
        created_at: '2026-10-01',
        relatedList: [],
        price: '',
        purchased: ''
      }))
      const episodes = [1, 2].map((order) => ({
        _id: 'e' + order,
        order,
        title: ['第1话 相遇的那一天', '第2话 风从远方来'][order - 1]
      }))
      const profile = {
        _id: 'u1',
        name: '测试读者',
        level: 12,
        levelName: '正式会员',
        exp: 1200,
        expPercent: 42,
        coin: 360,
        favouriteCount: 6,
        maxFavouriteCount: 100,
        isPunched: false
      }
      const state = window as any
      state.__calls = []
      state.__fail = ''
      state.isTauri = true
      state.__windowState = { maximized: false, minimized: false, closed: false }
      state.__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: 'main' } },
        invoke: async (cmd: string, args: any = {}) => {
          state.__calls.push({ cmd, args })
          if (state.__fail === cmd) throw '连接暂时不可用，请稍后重试'
          switch (cmd) {
            case 'plugin:window|is_maximized':
              return state.__windowState.maximized
            case 'plugin:window|toggle_maximize':
              state.__windowState.maximized = !state.__windowState.maximized
              window.dispatchEvent(new Event('resize'))
              return
            case 'plugin:window|minimize':
              state.__windowState.minimized = true
              return
            case 'plugin:window|close':
              state.__windowState.closed = true
              return
            case 'plugin:window|set_theme':
            case 'plugin:window|start_dragging':
              return
            case 'is_logged_in':
              return loggedIn
            case 'login':
              return profile
            case 'logout':
              return null
            case 'get_my_profile':
              return profile
            case 'get_daily':
              return {
                dailyId: '74',
                eventName: '11月-兔兔萬歲',
                currentProgress: '0%',
                threeDaysCoin: '30',
                threeDaysExp: '30',
                sevenDaysCoin: '500',
                sevenDaysExp: '500',
                records: []
              }
            case 'daily_checkin':
              return { status: 'ok', message: '签到成功', actionType: 'checkin' }
            case 'get_download_dir':
              return 'D:/JM/downloads'
            case 'get_proxy':
              return ''
            case 'get_api_domain':
              return 'www.cdnhth.net'
            case 'get_api_domains':
              return ['www.cdnhth.net', 'www.cdngw.net']
            case 'get_api_version':
              return '2.1.12'
            case 'get_img_host':
              return 'cdn-msp2.jmapiproxy2.cc'
            case 'get_download_settings':
              return { format: 'jpg', concurrency: 8, interval_ms: 300 }
            case 'save_download_settings':
            case 'set_proxy':
            case 'set_api_domain':
            case 'set_img_host':
            case 'set_download_dir':
            case 'save_setting':
              return null
            case 'refresh_api_domains':
              return ['www.cdnhth.net', 'www.cdngw.net']
            case 'get_image_cache_status':
              return { enabled: false, limit_mb: 256, used_bytes: 0 }
            case 'set_image_cache_settings':
            case 'clear_image_cache':
              return 0
            case 'get_promote':
              return [
                {
                  id: 'p1',
                  title: '本周推荐',
                  slug: '',
                  sectionType: 'promote',
                  filterValue: '1',
                  content: comics.slice(0, 4)
                }
              ]
            case 'get_promote_list':
              return { total: 4, list: comics.slice(0, 4) }
            case 'get_ranking':
            case 'get_latest_comics':
            case 'get_category_comics':
              return {
                searchQuery: '',
                searchType: 'ranking',
                total: 4,
                content: comics.slice(0, 4)
              }
            case 'get_weekly_info':
              return {
                categories: [{ id: '264', title: '', time: '2026第263期10.30 - 10.23' }],
                types: [
                  { id: 'manga', title: '日漫' },
                  { id: 'another', title: '其他' }
                ]
              }
            case 'get_weekly':
              return { total: 4, list: comics.slice(0, 4) }
            case 'get_categories':
              return {
                categories: [
                  { _id: '1', title: '同人', slug: 'doujin', totalAlbums: 100, subCategories: [] },
                  { _id: '2', title: '單本', slug: 'single', totalAlbums: 80, subCategories: [] }
                ],
                blocks: []
              }
            case 'search_comics':
              return {
                searchQuery: args.req.keyword,
                searchType: args.req.main_tag === 3 ? 'tag' : 'site',
                total: 4,
                content: comics.slice(0, 4)
              }
            case 'get_comic_bundle':
              return { comic: comics.find((c) => c._id === args.comicId) || comics[0], episodes }
            case 'get_forum':
              return {
                docs: [
                  {
                    _id: 'cm1',
                    content: '很喜欢这个安静的故事。',
                    likesCount: 8,
                    isSpoiler: false,
                    createdAt: '2026-11-01',
                    user: { _id: 'u2', name: '林间的风', level: 6 },
                    replies: []
                  }
                ],
                total: 1,
                pages: 1
              }
            case 'post_comment':
              return { status: 'ok', message: '评论已发布', actionType: 'comment' }
            case 'toggle_like':
            case 'toggle_favourite':
              return { status: 'ok', message: 'ok', actionType: 'toggle' }
            case 'get_my_favourites':
              return { list: comics.slice(0, 4), folders: [], total: 4, count: 4 }
            case 'get_watch_list':
              return { total: 4, list: comics.slice(0, 4) }
            case 'move_favourite':
              return { status: 'ok', message: 'ok', actionType: 'move' }
            case 'get_download_state_sets':
              return { history: [], local: [] }
            case 'get_downloaded_chapter_ids':
            case 'get_history_chapter_ids':
            case 'get_history_episode_orders':
              return []
            case 'get_local_pictures':
              return null
            case 'get_pictures_page':
              return {
                docs: [1, 2, 3].map((n) => ({
                  _id: '0000' + n + '.webp',
                  filename: '0000' + n + '.webp',
                  url: 'jmimg://photo/e1/0000' + n + '.webp'
                })),
                total: 3,
                pages: 1
              }
            case 'start_download':
              return 'task'
            case 'pause_download':
            case 'resume_download':
            case 'cancel_download':
            case 'clear_finished_download_tasks':
            case 'cleanup_library':
              return 0
            case 'get_download_queue':
              return [
                {
                  id: 'task1',
                  comic_id: 'c0',
                  comic_title: '雨后的书店',
                  chapter_id: 'e1',
                  episode_order: 1,
                  episode_title: '第1话 相遇的那一天',
                  status: 'downloading',
                  total_images: 10,
                  downloaded_images: 5,
                  save_path: 'D:/JM/downloads/雨后的书店',
                  created_at: '2026-11-02'
                }
              ]
            case 'get_download_history':
              return []
            case 'get_library':
              return []
            case 'get_library_stats':
              return { total_comics: 0, total_episodes: 0, total_images: 0 }
            case 'export_download_history':
            case 'import_download_history':
              return null
            default:
              return null
          }
        }
      }
    },
    { loggedIn }
  )
}

async function calls(page: Page): Promise<Call[]> {
  return page.evaluate(() => (window as any).__calls as Call[])
}

test('guest browsing loads discovery data', async ({ page }) => {
  await fixture(page, false)
  await page.goto('/')
  await expect(page.getByText('以游客身份浏览')).toBeVisible()
  await page.getByText('以游客身份浏览').click()
  await expect(page.locator('.page-header h1')).toHaveText('发现漫画')
  await expect(page.getByText('本周推荐')).toBeVisible()
  await expect(page.getByText('热门排行榜')).toBeVisible()
  await expect(page.getByText('雨后的书店').first()).toBeVisible()
  const invoked = await calls(page)
  expect(invoked.some((call) => call.cmd === 'get_promote')).toBe(true)
  expect(invoked.some((call) => call.cmd === 'get_ranking')).toBe(true)
  expect(invoked.some((call) => call.cmd === 'get_latest_comics')).toBe(true)
})

test('search supports site and tag modes', async ({ page }) => {
  await fixture(page, true)
  await page.goto('/search')
  await page.locator('input[aria-label="搜索关键词"]').fill('日常')
  await page.getByRole('button', { name: /搜索漫画/ }).click()
  await expect(page.getByText('雨后的书店').first()).toBeVisible()
  let invoked = await calls(page)
  const siteSearch = invoked.filter((call) => call.cmd === 'search_comics').pop()
  expect(siteSearch?.args.req.main_tag).toBe(0)

  await page.getByRole('tab', { name: '标签' }).click()
  await page.locator('input[aria-label="搜索关键词"]').fill('純愛')
  await page.getByRole('button', { name: /搜索漫画/ }).click()
  invoked = await calls(page)
  const tagSearch = invoked.filter((call) => call.cmd === 'search_comics').pop()
  expect(tagSearch?.args.req.main_tag).toBe(3)
  expect(tagSearch?.args.req.keyword).toContain('純愛')
})

test('comic detail renders chapters and comments', async ({ page }) => {
  await fixture(page, true)
  await page.goto('/comic/c0')
  await expect(page.locator('.detail-info h1')).toHaveText('雨后的书店')
  await expect(page.getByText('章节目录')).toBeVisible()
  await expect(page.getByText('第1话 相遇的那一天').first()).toBeVisible()
  await expect(page.getByText('评论区')).toBeVisible()
  await expect(page.getByText('很喜欢这个安静的故事。')).toBeVisible()
})

test('settings exposes JM lines and saves download settings', async ({ page }) => {
  await fixture(page, true)
  await page.goto('/settings')
  await expect(page.getByText('接口与图源')).toBeVisible()
  await expect(page.locator('select[aria-label="API 线路"]')).toHaveValue('www.cdnhth.net')
  await page.getByRole('button', { name: '保存下载设置' }).click()
  await expect(page.getByText('设置已保存')).toBeVisible()
  const invoked = await calls(page)
  expect(invoked.some((call) => call.cmd === 'save_download_settings')).toBe(true)
})

test('download task can be paused', async ({ page }) => {
  await fixture(page, true)
  await page.goto('/downloads')
  await expect(page.getByText('第1话 相遇的那一天')).toBeVisible()
  await page.locator('button[title="暂停"]').click()
  const invoked = await calls(page)
  expect(invoked.some((call) => call.cmd === 'pause_download')).toBe(true)
})

test('weekly issues split issue number and date range', async ({ page }) => {
  await fixture(page, true)
  await page.goto('/weekly')
  const firstIssue = page.locator('.category-option').first()
  await expect(firstIssue).toContainText('第263期')
  await expect(firstIssue).toContainText('2026 年')
  await expect(page.locator('.category-results-heading h2')).toContainText('第263期')
  await expect(page.locator('.category-aliases')).toContainText('2026 年')
  await expect(page.locator('.segmented')).toContainText('日漫')
  await expect(page.locator('.comic-grid .comic-card').first()).toBeVisible()
})

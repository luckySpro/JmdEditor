const config = require('../../config')
const { downloads } = require('../../utils/desktop')
const { messages, format } = require('../../utils/i18n')

Page({
  data: {
    t: messages('zh-CN'),
    downloads: downloads(),
    releasesUrl: config.RELEASES_URL,
    versionLabel: '',
    author: 'M&J Group',
  },

  onLoad() {
    const t = messages(getApp().globalData.lang)
    this.setData({ t, versionLabel: format(t.version, { version: config.DESKTOP_VERSION }) })
    wx.setNavigationBarTitle({ title: t.aboutTitle })
  },

  onShareAppMessage() {
    return { title: `JmdEditor · ${this.data.t.tagline}`, path: '/pages/about/about' }
  },

  copyLink(e) {
    const t = this.data.t
    wx.setClipboardData({
      data: e.currentTarget.dataset.url,
      success: () => wx.showToast({ title: t.copied, icon: 'success' }),
    })
  },
})

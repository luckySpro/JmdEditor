const api = require('../../utils/api')
const { messages, format } = require('../../utils/i18n')

const PAPERS = ['a4', 'letter']

Page({
  data: {
    t: messages('zh-CN'),
    url: '',
    markdown: '',
    html: '',
    title: '',
    loading: false,
    exporting: '',
    result: null,
    templateIndex: 0,
    paperIndex: 0,
    toc: true,
    paperLabels: ['A4', 'Letter'],
  },

  onLoad(query) {
    const lang = getApp().globalData.lang
    this.setData({ t: messages(lang) })
    try {
      const saved = wx.getStorageSync('jmd.export') || {}
      this.setData({
        templateIndex: Math.max(0, api.TEMPLATES.indexOf(saved.template)),
        paperIndex: Math.max(0, PAPERS.indexOf(saved.paper)),
        toc: saved.toc !== false,
      })
    } catch (e) {
      /* first run */
    }
    // Opened from a shared card: /pages/index/index?url=…
    if (query && query.url) {
      this.setData({ url: decodeURIComponent(query.url) })
      this.openUrl()
    }
  },

  onShareAppMessage() {
    return {
      title: this.data.title || 'Jmd Reader',
      path: this.data.url ? `/pages/index/index?url=${encodeURIComponent(this.data.url)}` : '/pages/index/index',
    }
  },

  onUrlInput(e) {
    this.setData({ url: e.detail.value })
  },

  options(formatName) {
    return {
      format: formatName,
      template: api.TEMPLATES[this.data.templateIndex],
      paper: PAPERS[this.data.paperIndex],
      toc: this.data.toc,
      numberHeadings: true,
      pageNumbers: true,
      lang: getApp().globalData.lang,
    }
  },

  showError(err) {
    const t = this.data.t
    const message = err && err.code === 'offline' ? t.network : (err && err.message) || String(err)
    wx.showModal({ title: t.errorTitle, content: message, showCancel: false })
  },

  async show(markdown) {
    this.setData({ markdown, title: api.titleOf(markdown), result: null })
    const html = await api.render(markdown, this.options('html'))
    this.setData({ html })
    wx.pageScrollTo({ scrollTop: 0, duration: 0 })
  },

  async openUrl() {
    const url = this.data.url.trim()
    if (!url || this.data.loading) return
    this.setData({ loading: true })
    try {
      const note = await api.importUrl(url)
      await this.show(note.markdown)
    } catch (err) {
      this.showError(err)
    } finally {
      this.setData({ loading: false })
    }
  },

  chooseFromChat() {
    const t = this.data.t
    wx.chooseMessageFile({
      count: 1,
      type: 'file',
      extension: ['md', 'markdown', 'txt'],
      success: (res) => {
        const file = res.tempFiles[0]
        if (!/\.(md|markdown|txt)$/i.test(file.name)) return this.showError({ message: t.notMarkdown })
        if (file.size > api.MAX_MARKDOWN_BYTES) return this.showError({ message: t.tooLarge })
        wx.getFileSystemManager().readFile({
          filePath: file.path,
          encoding: 'utf8',
          success: async (r) => {
            this.setData({ url: '', loading: true })
            try {
              await this.show(String(r.data).replace(/^﻿/, ''))
            } catch (err) {
              this.showError(err)
            } finally {
              this.setData({ loading: false })
            }
          },
          fail: (err) => this.showError({ message: err.errMsg }),
        })
      },
    })
  },

  onTemplate(e) {
    this.setData({ templateIndex: Number(e.detail.value) }, () => this.remember())
  },
  onPaper(e) {
    this.setData({ paperIndex: Number(e.detail.value) }, () => this.remember())
  },
  onToc(e) {
    this.setData({ toc: e.detail.value }, () => this.remember())
  },
  remember() {
    try {
      wx.setStorageSync('jmd.export', {
        template: api.TEMPLATES[this.data.templateIndex],
        paper: PAPERS[this.data.paperIndex],
        toc: this.data.toc,
      })
    } catch (e) {
      /* storage full: settings last for this session */
    }
  },

  exportPdf() {
    this.exportAs('pdf')
  },
  exportWord() {
    this.exportAs('docx')
  },

  async exportAs(formatName) {
    if (!this.data.markdown || this.data.exporting) return
    this.setData({ exporting: formatName, result: null })
    try {
      const name = (this.data.title || 'document').slice(0, 60)
      const file = await api.exportFile(this.data.markdown, this.options(formatName), `${name}.md`)
      this.setData({
        result: {
          path: file.path,
          name: file.name,
          kind: formatName === 'pdf' ? 'PDF' : 'DOCX',
          label: format(this.data.t.ready, { name: file.name }),
          size: api.formatSize(file.size),
          fileType: formatName,
        },
      })
    } catch (err) {
      this.showError(err)
    } finally {
      this.setData({ exporting: '' })
    }
  },

  openResult() {
    const r = this.data.result
    if (!r) return
    wx.openDocument({ filePath: r.path, fileType: r.fileType, showMenu: true, fail: (err) => this.showError({ message: err.errMsg }) })
  },

  sendResult() {
    const r = this.data.result
    if (!r) return
    wx.shareFileMessage({ filePath: r.path, fileName: r.name, fail: (err) => this.showError({ message: err.errMsg }) })
  },
})

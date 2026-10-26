const { detectLang, messages } = require('./utils/i18n')

App({
  globalData: { lang: 'zh-CN' },
  onLaunch() {
    let language = 'zh_CN'
    try {
      language = (wx.getAppBaseInfo ? wx.getAppBaseInfo() : wx.getSystemInfoSync()).language || language
    } catch (e) {
      /* keep the default */
    }
    this.globalData.lang = detectLang(language)
    this.requireLatestVersion()
  },

  // WeChat downloads a new version in the background; once it is ready the
  // user must restart into it (the dialog has no cancel button).
  requireLatestVersion() {
    if (!wx.getUpdateManager) return
    const t = messages(this.globalData.lang)
    const manager = wx.getUpdateManager()
    manager.onUpdateReady(() => {
      wx.showModal({
        title: t.updateTitle,
        content: t.updateReady,
        showCancel: false,
        confirmText: t.updateRestart,
        success: () => manager.applyUpdate(),
      })
    })
    manager.onUpdateFailed(() => {
      wx.showModal({ title: t.updateTitle, content: t.updateFailed, showCancel: false })
    })
  },
})

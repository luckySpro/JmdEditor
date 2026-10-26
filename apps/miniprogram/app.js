const { detectLang } = require('./utils/i18n')

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
  },
})

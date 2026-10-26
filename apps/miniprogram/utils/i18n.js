// Interface text in the four languages the desktop app ships.

const MESSAGES = {
  'zh-CN': {
    openLabel: '从链接或聊天文件打开',
    open: '打开',
    chooseChat: '从聊天中选择 .md 文件',
    placeholder: '粘贴 GitHub、Gitee、Gist 或网页链接',
    loading: '正在获取…',
    exportPdf: '导出 PDF',
    exportWord: '导出 Word',
    exporting: '正在导出…',
    ready: '{name} 已生成',
    openFile: '打开',
    sendToChat: '发送给朋友',
    empty: '打开一篇 Markdown 笔记，即可在这里阅读，并导出为 PDF 或 Word。',
    template: '模板',
    paper: '纸张',
    toc: '目录',
    templates: ['简洁', '学术', '商务', 'GitHub'],
    errorTitle: '出现问题',
    notMarkdown: '请选择 .md 或 .markdown 文件',
    tooLarge: '文件超过 5 MB',
    network: '无法连接导出服务，请检查网络或服务器域名设置。',
  },
  'zh-TW': {
    openLabel: '從連結或聊天檔案開啟',
    open: '開啟',
    chooseChat: '從聊天中選擇 .md 檔案',
    placeholder: '貼上 GitHub、Gitee、Gist 或網頁連結',
    loading: '正在取得…',
    exportPdf: '匯出 PDF',
    exportWord: '匯出 Word',
    exporting: '正在匯出…',
    ready: '{name} 已產生',
    openFile: '開啟',
    sendToChat: '傳送給朋友',
    empty: '開啟一篇 Markdown 筆記，即可在這裡閱讀，並匯出為 PDF 或 Word。',
    template: '範本',
    paper: '紙張',
    toc: '目錄',
    templates: ['簡潔', '學術', '商務', 'GitHub'],
    errorTitle: '發生問題',
    notMarkdown: '請選擇 .md 或 .markdown 檔案',
    tooLarge: '檔案超過 5 MB',
    network: '無法連線匯出服務，請檢查網路或伺服器網域設定。',
  },
  ja: {
    openLabel: 'リンクまたはチャットのファイルから開く',
    open: '開く',
    chooseChat: 'チャットから .md ファイルを選ぶ',
    placeholder: 'GitHub、Gitee、Gist または Web ページのリンク',
    loading: '取得しています…',
    exportPdf: 'PDF に書き出す',
    exportWord: 'Word に書き出す',
    exporting: '書き出しています…',
    ready: '{name} ができました',
    openFile: '開く',
    sendToChat: '友だちに送る',
    empty: 'Markdown ノートを開くと、ここで読んで PDF や Word に書き出せます。',
    template: 'テンプレート',
    paper: '用紙',
    toc: '目次',
    templates: ['クリーン', 'アカデミック', 'ビジネス', 'GitHub'],
    errorTitle: '問題が発生しました',
    notMarkdown: '.md または .markdown ファイルを選んでください',
    tooLarge: 'ファイルが 5 MB を超えています',
    network: '書き出しサービスに接続できません。ネットワークとサーバードメインの設定を確認してください。',
  },
  en: {
    openLabel: 'Open from a link or a chat file',
    open: 'Open',
    chooseChat: 'Choose a .md file from a chat',
    placeholder: 'Paste a GitHub, Gitee, gist or web link',
    loading: 'Fetching…',
    exportPdf: 'Export PDF',
    exportWord: 'Export Word',
    exporting: 'Exporting…',
    ready: '{name} is ready',
    openFile: 'Open',
    sendToChat: 'Send to chat',
    empty: 'Open a Markdown note to read it here and export it as PDF or Word.',
    template: 'Template',
    paper: 'Paper',
    toc: 'Contents',
    templates: ['Clean', 'Academic', 'Business', 'GitHub'],
    errorTitle: 'Something went wrong',
    notMarkdown: 'Choose a .md or .markdown file',
    tooLarge: 'The file is larger than 5 MB',
    network: 'The export service could not be reached. Check the network and the server domain settings.',
  },
}

/** Maps WeChat's `zh_CN`, `zh_TW`, `zh_HK`, `ja`, `en` … to our language codes. */
function detectLang(language) {
  const l = String(language || '').toLowerCase().replace('_', '-')
  if (l.startsWith('zh')) return /tw|hk|mo|hant/.test(l) ? 'zh-TW' : 'zh-CN'
  if (l.startsWith('ja')) return 'ja'
  if (l.startsWith('en')) return 'en'
  return 'zh-CN'
}

function messages(lang) {
  return MESSAGES[lang] || MESSAGES['zh-CN']
}

function format(text, vars) {
  return String(text).replace(/\{(\w+)\}/g, (_, k) => (vars && k in vars ? vars[k] : `{${k}}`))
}

module.exports = { MESSAGES, detectLang, messages, format }

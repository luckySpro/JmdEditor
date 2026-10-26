// Calls to jmd-server. Exports come back as bytes and are written to the
// mini-program's user data folder, then opened or shared from there.

const { API_BASE } = require('../config')

const MAX_MARKDOWN_BYTES = 5 * 1024 * 1024
const TEMPLATES = ['clean', 'academic', 'business', 'github']

function errorFrom(res) {
  try {
    const data = typeof res.data === 'string' ? JSON.parse(res.data) : res.data
    if (data && data.code) return data
    if (res.data instanceof ArrayBuffer) {
      const text = decodeUtf8(res.data)
      const parsed = JSON.parse(text)
      if (parsed && parsed.code) return parsed
    }
  } catch (e) {
    /* not one of our errors */
  }
  return { code: 'server', message: `HTTP ${res.statusCode}` }
}

function decodeUtf8(buffer) {
  const bytes = new Uint8Array(buffer)
  let out = ''
  for (let i = 0; i < bytes.length; i++) out += '%' + bytes[i].toString(16).padStart(2, '0')
  try {
    return decodeURIComponent(out)
  } catch (e) {
    return ''
  }
}

function request(path, data, responseType) {
  return new Promise((resolve, reject) => {
    wx.request({
      url: API_BASE.replace(/\/$/, '') + path,
      method: 'POST',
      data,
      header: { 'content-type': 'application/json' },
      responseType: responseType || 'text',
      timeout: 90000,
      success(res) {
        if (res.statusCode >= 200 && res.statusCode < 300) resolve(res)
        else reject(errorFrom(res))
      },
      fail(err) {
        reject({ code: 'offline', message: err.errMsg || 'request failed' })
      },
    })
  })
}

function parseJson(data) {
  return typeof data === 'string' ? JSON.parse(data) : data
}

function importUrl(url) {
  return request('/api/import', { url, recordSource: true }).then((res) => parseJson(res.data))
}

function render(markdown, options) {
  return request('/api/render', { markdown, options, mini: true }).then((res) => String(res.data))
}

/** File name from `Content-Disposition`, preferring the UTF-8 form. */
function fileNameFromDisposition(header, fallback) {
  if (!header) return fallback
  const star = /filename\*=UTF-8''([^;]+)/i.exec(header)
  if (star) {
    try {
      return decodeURIComponent(star[1].trim())
    } catch (e) {
      /* fall through */
    }
  }
  const plain = /filename="?([^";]+)"?/i.exec(header)
  return plain ? plain[1].trim() : fallback
}

function headerValue(headers, name) {
  if (!headers) return ''
  const key = Object.keys(headers).find((k) => k.toLowerCase() === name)
  return key ? headers[key] : ''
}

/** Exports and saves the file; resolves with `{ path, name, size }`. */
function exportFile(markdown, options, fileName) {
  return request('/api/export', { markdown, options, fileName }, 'arraybuffer').then(
    (res) =>
      new Promise((resolve, reject) => {
        const ext = options.format === 'docx' ? 'docx' : options.format
        const name = fileNameFromDisposition(headerValue(res.header, 'content-disposition'), `document.${ext}`)
        const path = `${wx.env.USER_DATA_PATH}/${name}`
        wx.getFileSystemManager().writeFile({
          filePath: path,
          data: res.data,
          success: () => resolve({ path, name, size: res.data.byteLength }),
          fail: (err) => reject({ code: 'io', message: err.errMsg }),
        })
      }),
  )
}

function formatSize(bytes) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

/** Title from the first `# ` heading or the front matter, for file names. */
function titleOf(markdown) {
  const fm = /^---\r?\n([\s\S]*?)\r?\n---/.exec(markdown)
  const t = fm && /^title:\s*(.+)$/m.exec(fm[1])
  if (t) return t[1].trim().replace(/^(['"])(.*)\1$/, '$2')
  const h = /^#\s+(.+)$/m.exec(markdown.replace(/```[\s\S]*?```/g, ''))
  return h ? h[1].replace(/[*_`]/g, '').trim() : ''
}

module.exports = {
  MAX_MARKDOWN_BYTES,
  TEMPLATES,
  importUrl,
  render,
  exportFile,
  fileNameFromDisposition,
  formatSize,
  titleOf,
  decodeUtf8,
}

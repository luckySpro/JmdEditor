// Address of your jmd-server. WeChat only calls HTTPS domains that are listed
// under 开发管理 → 开发设置 → 服务器域名 (request and downloadFile) in the
// mini-program admin console. In the developer tools you can untick
// "不校验合法域名" to try a local server first.
module.exports = {
  API_BASE: 'https://export.example.com',
  // Desktop downloads shown on the About page. Mini-programs cannot open
  // outside links, so the page copies them for the user to paste in a browser.
  // Keep DESKTOP_VERSION in step with the latest published GitHub release.
  RELEASES_URL: 'https://github.com/luckySpro/JmdEditor/releases',
  DESKTOP_VERSION: '0.2.0',
}

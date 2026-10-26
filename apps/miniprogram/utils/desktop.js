// Download links for the desktop app, named the way tauri-action uploads them.
const config = require('../config')

function downloads(version = config.DESKTOP_VERSION, base = config.RELEASES_URL) {
  const at = (file) => `${base}/download/v${version}/${file}`
  return [
    { id: 'windows', name: 'Windows', detail: '64-bit · .exe', url: at(`JmdEditor_${version}_x64-setup.exe`) },
    { id: 'macos', name: 'macOS', detail: 'Apple silicon + Intel · .dmg', url: at(`JmdEditor_${version}_universal.dmg`) },
    { id: 'linux', name: 'Linux', detail: 'x64 · .AppImage', url: at(`JmdEditor_${version}_amd64.AppImage`) },
  ]
}

module.exports = { downloads }

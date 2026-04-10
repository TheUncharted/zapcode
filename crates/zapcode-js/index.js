const { existsSync, readFileSync } = require('fs')
const { join } = require('path')

const { platform, arch } = process

function isMusl() {
  if (!process.report || typeof process.report.getReport !== 'function') {
    try {
      const lddPath = require('child_process').execSync('which ldd').toString().trim()
      return readFileSync(lddPath, 'utf8').includes('musl')
    } catch {
      return true
    }
  } else {
    const report = process.report.getReport()
    const rpt = typeof report === 'string' ? JSON.parse(report) : report
    return !rpt.header.glibcVersionRuntime
  }
}

function getPlatformKey() {
  let key = `${platform}-${arch}`
  if (platform === 'linux') {
    key = `linux-${arch === 'x64' ? 'x64' : 'arm64'}-${isMusl() ? 'musl' : 'gnu'}`
  } else if (platform === 'darwin') {
    key = `darwin-${arch === 'arm64' ? 'arm64' : 'x64'}`
  } else if (platform === 'win32') {
    key = `win32-${arch === 'arm64' ? 'arm64' : 'x64'}-msvc`
  }
  return key
}

function loadNativeBinding() {
  const key = getPlatformKey()
  const localFile = `zapcode.${key}.node`
  const npmPkg = `@unchartedfr/zapcode-${key}`

  // Try local .node file first (development)
  const localPath = join(__dirname, localFile)
  if (existsSync(localPath)) {
    return require(localPath)
  }

  // Try platform-specific npm package (production install)
  try {
    return require(npmPkg)
  } catch {
    throw new Error(
      `Failed to load native binding for ${platform}-${arch}.\n` +
      `Looked for: ${localFile} (local) or ${npmPkg} (npm package).\n` +
      `Try reinstalling: npm install @unchartedfr/zapcode`
    )
  }
}

const binding = loadNativeBinding()

module.exports = binding
module.exports.Zapcode = binding.Zapcode
module.exports.ZapcodeSessionHandle = binding.ZapcodeSessionHandle
module.exports.ZapcodeSnapshotHandle = binding.ZapcodeSnapshotHandle

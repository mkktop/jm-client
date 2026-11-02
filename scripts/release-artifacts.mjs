import { copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import path from 'node:path'

const platforms = {
  'windows-x64': {
    target: 'x86_64-pc-windows-msvc',
    files: [
      ['nsis', '_x64-setup.exe'],
      ['msi', '_x64_en-US.msi']
    ]
  },
  'macos-arm64': { target: 'aarch64-apple-darwin', files: [['dmg', '_aarch64.dmg']] }
}
const version = JSON.parse(readFileSync('package.json', 'utf8')).version
const git = (ref) => execFileSync('git', ['rev-parse', ref], { encoding: 'utf8' }).trim()
const commit = git('HEAD')
const tree = git('HEAD^{tree}')
const digest = (data) => createHash('sha256').update(data).digest('hex')

function verifyFile(name, data) {
  if (data.length < 1024 * 1024 || data.length > 512 * 1024 * 1024)
    throw new Error(`Invalid installer size: ${name}`)
  const valid = name.endsWith('.exe')
    ? data.subarray(0, 2).toString() === 'MZ'
    : name.endsWith('.msi')
      ? data.subarray(0, 8).equals(Buffer.from('d0cf11e0a1b11ae1', 'hex'))
      : name.endsWith('.dmg') && data.subarray(-512, -508).toString() === 'koly'
  if (!valid) throw new Error(`Invalid installer format: ${name}`)
}

function stage(platform) {
  const config = platforms[platform]
  if (!config) throw new Error('Unknown platform')
  const directory = path.join('artifacts', 'installers', platform)
  mkdirSync(directory, { recursive: true })
  const installers = config.files.map(([folder, suffix]) => {
    const sourceDir = path.join('src-tauri', 'target', config.target, 'release', 'bundle', folder)
    const matches = readdirSync(sourceDir).filter((name) => name.endsWith(`_${version}${suffix}`))
    if (matches.length !== 1) throw new Error(`Expected one ${platform} ${suffix} installer`)
    const source = path.join(sourceDir, matches[0])
    const filename = matches[0].replaceAll(' ', '.')
    const data = readFileSync(source)
    verifyFile(filename, data)
    copyFileSync(source, path.join(directory, filename))
    return { filename, size: data.length, sha256: digest(data) }
  })
  writeFileSync(
    path.join(directory, 'build-info.json'),
    JSON.stringify(
      { platform, version, sourceCommit: commit, sourceTree: tree, installers },
      null,
      2
    ) + '\n'
  )
  console.log(`Staged verified ${platform} installers from ${commit}`)
}

function assemble() {
  const files = []
  for (const [platform, config] of Object.entries(platforms)) {
    const directory = path.join('installers', `${platform}-installer`)
    const manifest = JSON.parse(readFileSync(path.join(directory, 'build-info.json'), 'utf8'))
    if (
      manifest.platform !== platform ||
      manifest.version !== version ||
      manifest.sourceCommit !== commit ||
      manifest.sourceTree !== tree ||
      manifest.installers.length !== config.files.length
    )
      throw new Error(`Installer provenance mismatch: ${platform}`)
    for (const [, suffix] of config.files) {
      const matches = manifest.installers.filter(
        (file) =>
          path.basename(file.filename) === file.filename &&
          file.filename.endsWith(`_${version}${suffix}`)
      )
      if (matches.length !== 1) throw new Error(`Missing ${platform} ${suffix} installer`)
      const file = matches[0]
      const data = readFileSync(path.join(directory, file.filename))
      verifyFile(file.filename, data)
      if (data.length !== file.size || digest(data) !== file.sha256)
        throw new Error(`Installer checksum mismatch: ${file.filename}`)
      files.push({ ...file, data })
    }
  }
  // Every platform must pass validation before any release file is staged.
  mkdirSync('release', { recursive: true })
  for (const file of files) writeFileSync(path.join('release', file.filename), file.data)
  writeFileSync(
    'release/SHA256SUMS.txt',
    files.map((file) => `${file.sha256}  ${file.filename}`).join('\n') + '\n'
  )
  console.log(`Verified ${files.length} installers against source ${commit}`)
}

if (process.argv[2] === 'stage') stage(process.argv[3])
else if (process.argv[2] === 'assemble') assemble()
else throw new Error('Usage: release-artifacts.mjs stage <platform> | assemble')

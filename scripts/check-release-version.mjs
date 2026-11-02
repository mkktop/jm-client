import { readFileSync } from 'node:fs'

const readJson = (file) => JSON.parse(readFileSync(file, 'utf8'))
const tag = process.argv[2] || process.env.RELEASE_TAG
if (!/^v\d+\.\d+\.\d+$/.test(tag || '')) throw new Error('A stable release tag is required')
const expected = tag.slice(1)
const cargoVersion = readFileSync('src-tauri/Cargo.toml', 'utf8').match(
  /^version = "([^"]+)"/m
)?.[1]
const lockVersion = readFileSync('src-tauri/Cargo.lock', 'utf8').match(
  /name = "app"\nversion = "([^"]+)"/
)?.[1]
const versions = [
  readJson('package.json').version,
  readJson('package-lock.json').packages[''].version,
  readJson('src-tauri/tauri.conf.json').version,
  cargoVersion,
  lockVersion
]
if (versions.some((version) => version !== expected))
  throw new Error('Release tag and all application versions must match')
console.log(`Verified application version ${expected}`)

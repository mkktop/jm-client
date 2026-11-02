import { copyFileSync } from 'node:fs'

for (const [source, destination] of [
  ['../src-tauri/icons/128x128@2x.png', '../src/assets/jm-brand.png'],
  ['../src-tauri/icons/64x64.png', '../public/favicon.png']
]) {
  copyFileSync(new URL(source, import.meta.url), new URL(destination, import.meta.url))
}

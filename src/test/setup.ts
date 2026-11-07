import '@testing-library/jest-dom/vitest'

// Node 25+ ships an experimental global `localStorage`/`sessionStorage` that is undefined unless
// `--localstorage-file` is passed, and it shadows the jsdom ones vitest would otherwise expose.
// Point the globals back at jsdom's storage so tests behave the same on every Node version.
const dom = (globalThis as unknown as { jsdom?: { window: Window } }).jsdom
for (const key of ['localStorage', 'sessionStorage'] as const) {
  const storage = dom?.window?.[key]
  if (storage && globalThis[key] !== storage) {
    Object.defineProperty(globalThis, key, { value: storage, configurable: true, writable: true })
  }
}

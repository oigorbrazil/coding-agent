import { beforeEach, describe, expect, it } from 'vitest'

import {
  getStorageNamespace,
  readScopedStorage,
  removeScopedStorage,
  restoreScopedStorage,
  scopedStorageKey,
  scopedStorageSnapshot,
  setStorageNamespace,
  writeScopedStorage,
} from './storageNamespace'

beforeEach(() => {
  localStorage.clear()
  setStorageNamespace('default')
})

describe('scopedStorageKey', () => {
  it('usa o namespace ativo', () => {
    expect(scopedStorageKey('foo')).toBe('coding-agent:default:foo')
    setStorageNamespace('work')
    expect(scopedStorageKey('foo')).toBe('coding-agent:work:foo')
    expect(getStorageNamespace()).toBe('work')
  })

  it('usa default para namespace em branco', () => {
    setStorageNamespace('   ')
    expect(scopedStorageKey('foo')).toBe('coding-agent:default:foo')
  })
})

describe('readScopedStorage', () => {
  it('retorna o valor do namespace quando existe', () => {
    localStorage.setItem('coding-agent:default:foo', 'current')
    expect(readScopedStorage('foo')).toBe('current')
  })

  it('lê e copia o formato legacy coding-agent quando permitido', () => {
    localStorage.setItem('coding-agent:foo', 'legacy')
    expect(readScopedStorage('foo', true)).toBe('legacy')
    expect(localStorage.getItem('coding-agent:default:foo')).toBe('legacy')
  })

  it('lê e copia o formato antigo ensemble quando permitido', () => {
    localStorage.setItem('ensemble:foo', 'ancient')
    expect(readScopedStorage('foo', true)).toBe('ancient')
    expect(localStorage.getItem('coding-agent:default:foo')).toBe('ancient')
  })

  it('ignora chaves legacy quando allowLegacy é falso', () => {
    localStorage.setItem('coding-agent:foo', 'legacy')
    expect(readScopedStorage('foo')).toBeNull()
    expect(localStorage.getItem('coding-agent:default:foo')).toBeNull()
  })

  it('não herda dados legacy fora do namespace default', () => {
    localStorage.setItem('coding-agent:foo', 'legacy')
    setStorageNamespace('work')
    expect(readScopedStorage('foo', true)).toBeNull()
  })

  it('retorna null quando nenhuma chave existe', () => {
    expect(readScopedStorage('missing', true)).toBeNull()
  })
})

describe('writeScopedStorage and removeScopedStorage', () => {
  it('grava e remove no namespace ativo', () => {
    writeScopedStorage('foo', 'value')
    expect(localStorage.getItem('coding-agent:default:foo')).toBe('value')
    removeScopedStorage('foo')
    expect(localStorage.getItem('coding-agent:default:foo')).toBeNull()
  })
})

describe('scopedStorageSnapshot', () => {
  it("keeps one profile's keys, without their prefix", () => {
    localStorage.setItem('coding-agent:default:promptHistory:pty-1', '["npm test"]')
    localStorage.setItem('coding-agent:default:preferences.collapsedSections', '{"app-info":true}')
    localStorage.setItem('coding-agent:work:promptHistory:pty-2', '["ls"]')
    localStorage.setItem('coding-agent:bootAppearance', '{}')

    expect(scopedStorageSnapshot()).toEqual({
      'promptHistory:pty-1': '["npm test"]',
      'preferences.collapsedSections': '{"app-info":true}',
    })
    expect(scopedStorageSnapshot('work')).toEqual({ 'promptHistory:pty-2': '["ls"]' })
  })
})

describe('restoreScopedStorage', () => {
  it('writes the entries under the active profile', () => {
    setStorageNamespace('work')
    expect(restoreScopedStorage({ 'promptHistory:pty-1': '["npm test"]' })).toBe(1)
    expect(localStorage.getItem('coding-agent:work:promptHistory:pty-1')).toBe('["npm test"]')
    expect(localStorage.getItem('coding-agent:default:promptHistory:pty-1')).toBeNull()
  })
})

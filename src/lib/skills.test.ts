import { describe, expect, it } from 'vitest'

import {
  countSkills,
  groupSkillsByName,
  hasBundledUpdate,
  installedBundledAgents,
  matchesSkillQuery,
  pendingBundledAgents,
  splitBundledReport,
} from './skills'
import type {
  BundledSkill,
  BundledSkillState,
  SkillAgentSnapshot,
  SkillSummary,
} from './tauri/skills'

function skill(agent: string, name: string, overrides: Partial<SkillSummary> = {}): SkillSummary {
  return {
    name,
    agent,
    path: `C:/${agent}/skills/${name}`,
    resolvedPath: `C:/${agent}/skills/${name}`,
    description: '',
    linked: false,
    shared: false,
    bundled: false,
    fromCodingAgent: false,
    entryCount: 1,
    ...overrides,
  }
}

function snapshot(agent: string, skills: SkillSummary[]): SkillAgentSnapshot {
  return { agent, root: `C:/${agent}/skills`, exists: true, skills }
}

describe('groupSkillsByName', () => {
  it('merges the same skill across agents', () => {
    const groups = groupSkillsByName([
      snapshot('claude', [skill('claude', 'design'), skill('claude', 'lousa')]),
      snapshot('codex', [skill('codex', 'design')]),
    ])

    expect(groups.map((group) => group.name)).toEqual(['design', 'lousa'])
    expect(groups[0].agents).toEqual(['claude', 'codex'])
  })

  it('never offers the shared copy for a bulk remove', () => {
    const groups = groupSkillsByName([
      snapshot('claude', [skill('claude', 'design', { linked: true, shared: true })]),
      snapshot('shared', [skill('shared', 'design')]),
    ])

    expect(groups[0].agents).toEqual(['claude'])
    expect(groups[0].removable.map((entry) => entry.agent)).toEqual(['claude'])
    expect(groups[0].sharedEntry?.agent).toBe('shared')
  })

  it('excludes a bundled skill from the removable set', () => {
    const groups = groupSkillsByName([
      snapshot('codex', [skill('codex', 'imagegen', { bundled: true })]),
      snapshot('claude', [skill('claude', 'imagegen')]),
    ])

    expect(groups[0].removable.map((entry) => entry.agent)).toEqual(['claude'])
    expect(groups[0].bundled).toBe(false)
  })

  it('marks a group bundled only when every agent copy is bundled', () => {
    const groups = groupSkillsByName([
      snapshot('codex', [skill('codex', 'imagegen', { bundled: true })]),
    ])
    expect(groups[0].bundled).toBe(true)
    expect(groups[0].removable).toEqual([])
  })

  it('takes the first non-empty description it finds', () => {
    const groups = groupSkillsByName([
      snapshot('claude', [skill('claude', 'design')]),
      snapshot('codex', [skill('codex', 'design', { description: 'Brand system' })]),
    ])
    expect(groups[0].description).toBe('Brand system')
  })

  it('counts groups, not per-agent copies', () => {
    const snapshots = [
      snapshot('claude', [skill('claude', 'design')]),
      snapshot('codex', [skill('codex', 'design')]),
    ]
    expect(countSkills(snapshots)).toBe(1)
    expect(countSkills([])).toBe(0)
  })
})

describe('matchesSkillQuery', () => {
  const [group] = groupSkillsByName([
    snapshot('claude', [skill('claude', 'brand', { description: 'Diffusion Studio system' })]),
  ])

  it('matches on name and on description', () => {
    expect(matchesSkillQuery(group, 'BRA')).toBe(true)
    expect(matchesSkillQuery(group, 'diffusion')).toBe(true)
  })

  it('accepts an empty query and rejects a non-match', () => {
    expect(matchesSkillQuery(group, '  ')).toBe(true)
    expect(matchesSkillQuery(group, 'pptx')).toBe(false)
  })
})

function bundled(states: Record<string, BundledSkillState>): BundledSkill {
  return {
    name: 'coding',
    description: '',
    version: 1,
    repository: 'https://github.com/oigorbrazil/coding-opus',
    body: '',
    agents: Object.entries(states).map(([agent, state]) => ({
      agent,
      state,
      path: null,
      error: null,
    })),
  }
}

describe('bundled skills', () => {
  it('marks a group as from Coding Agent when any copy carries the marker', () => {
    const groups = groupSkillsByName([
      snapshot('claude', [skill('claude', 'coding', { fromCodingAgent: true })]),
      snapshot('codex', [skill('codex', 'design')]),
    ])

    expect(groups.map((group) => group.fromCodingAgent)).toEqual([true, false])
    expect(groups[0].removable).toHaveLength(1)
  })

  it('installs only where the skill is missing or outdated', () => {
    expect(pendingBundledAgents(bundled({ claude: 'available', codex: 'update' }))).toEqual([
      'claude',
      'codex',
    ])
    expect(pendingBundledAgents(bundled({ claude: 'conflict', codex: 'installed' }))).toEqual([])
  })

  it('reports an update only when a copy is outdated', () => {
    expect(hasBundledUpdate(bundled({ claude: 'installed', codex: 'update' }))).toBe(true)
    expect(hasBundledUpdate(bundled({ claude: 'installed', codex: 'available' }))).toBe(false)
  })

  it('removes only the copies Coding Agent installed', () => {
    expect(installedBundledAgents(bundled({ claude: 'installed', codex: 'update' }))).toEqual([
      'claude',
      'codex',
    ])
    expect(installedBundledAgents(bundled({ claude: 'conflict', codex: 'available' }))).toEqual([])
  })

  it('separates the agents a report reached from the ones that failed', () => {
    const report = bundled({ claude: 'installed', codex: 'available' }).agents
    report[1].error = 'write_failed:denied'
    const { done, failed } = splitBundledReport(report, (entry) => entry.state === 'installed')
    expect(done.map((entry) => entry.agent)).toEqual(['claude'])
    expect(failed.map((entry) => entry.agent)).toEqual(['codex'])
  })
})

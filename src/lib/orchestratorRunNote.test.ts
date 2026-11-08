import { describe, expect, it } from 'vitest'

import { isRunSettled, runNote } from './orchestratorRunNote'
import { groupRuns } from './orchestratorRuns'
import type { OrchestratorJob } from './tauri'

function job(
  partial: Partial<OrchestratorJob> & Pick<OrchestratorJob, 'id' | 'runId'>,
): OrchestratorJob {
  return {
    plannerId: null,
    agent: 'codex',
    runLabel: 'refactor payments',
    rules: null,
    spec: 'spec',
    cwd: 'C:/code/shop',
    status: 'done',
    threadId: null,
    outcome: 'succeeded',
    seconds: 12.4,
    plan: [],
    tokens: null,
    costUsd: null,
    quota: null,
    routing: null,
    worktree: null,
    role: null,
    model: null,
    effort: null,
    readOnly: false,
    pendingApproval: null,
    hasDiff: false,
    summary: '',
    ...partial,
  } as OrchestratorJob
}

describe('runNote', () => {
  it('files the run under its project with workers, totals and results', () => {
    const [run] = groupRuns([
      job({
        id: 'job-01',
        runId: 'run-01',
        agent: 'claude',
        model: 'haiku',
        effort: 'low',
        tokens: { total: { totalTokens: 1200 } } as OrchestratorJob['tokens'],
        costUsd: 0.004,
        summary: 'Added a docstring | nothing else',
      }),
      job({
        id: 'job-02',
        runId: 'run-01',
        tokens: { total: { totalTokens: 800 } } as OrchestratorJob['tokens'],
      }),
    ])
    const note = runNote(run)
    expect(note.title).toBe('refactor payments')
    expect(note.folder).toBe('30-Projetos/shop')
    expect(note.tags).toEqual(['orquestracao', 'shop'])
    expect(note.summary).toBe('2 worker(s) · 2,000 tokens · $0.0040 · 25 s of worker time')
    expect(note.body).toContain(
      '| job-01 | claude / haiku / low | done (succeeded) | 1200 | $0.0040 | 12 s |',
    )
    expect(note.body).toContain('| job-02 | codex | done (succeeded) | 800 | — | 12 s |')
    expect(note.body).toContain('- Result: Added a docstring | nothing else')
  })

  it('says when no provider reported a price', () => {
    const [run] = groupRuns([job({ id: 'job-01', runId: 'run-01' })])
    expect(runNote(run).summary).toContain('no price reported')
  })
})

describe('isRunSettled', () => {
  it('waits until no worker is running, queued or blocked', () => {
    const [busy] = groupRuns([
      job({ id: 'job-01', runId: 'run-01' }),
      job({ id: 'job-02', runId: 'run-01', status: 'running' }),
    ])
    expect(isRunSettled(busy)).toBe(false)
    const [done] = groupRuns([
      job({ id: 'job-01', runId: 'run-01' }),
      job({ id: 'job-02', runId: 'run-01', status: 'failed' }),
    ])
    expect(isRunSettled(done)).toBe(true)
  })
})

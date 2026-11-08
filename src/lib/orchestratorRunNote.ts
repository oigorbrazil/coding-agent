import { fmtUsd } from './costFormat'
import type { OrchestratorRun } from './orchestratorRuns'
import { basename } from './paths'
import type { SecondBrainRecordRequest } from './tauri'

const SPEC_LIMIT = 280
const SUMMARY_LIMIT = 600

function clip(text: string, limit: number): string {
  const flat = text.replace(/\s+/g, ' ').trim()
  return flat.length > limit ? `${flat.slice(0, limit - 1)}…` : flat
}

function cell(text: string): string {
  return text.replace(/\|/g, '\\|')
}

/** A run's settled lanes: a note is only worth writing once nothing in it can still change. */
export function isRunSettled(run: OrchestratorRun): boolean {
  return run.counts.running === 0 && run.counts.queued === 0 && run.counts.blocked === 0
}

/** The second brain note for one delegation run: who ran what, what came back and what it cost. */
export function runNote(run: OrchestratorRun): SecondBrainRecordRequest {
  const project = run.jobs.map((job) => basename(job.cwd)).find((name) => name.length > 0) ?? ''
  const totalTokens = run.jobs.reduce((sum, job) => sum + (job.tokens?.total?.totalTokens ?? 0), 0)
  const priced = run.jobs.filter((job) => job.costUsd !== null)
  const cost = priced.reduce((sum, job) => sum + (job.costUsd ?? 0), 0)
  const seconds = run.jobs.reduce((sum, job) => sum + (job.seconds ?? 0), 0)

  const summary = [
    `${run.jobs.length} worker(s)`,
    `${totalTokens.toLocaleString('en-US')} tokens`,
    priced.length > 0 ? `${fmtUsd(cost)}` : 'no price reported',
    `${Math.round(seconds)} s of worker time`,
  ].join(' · ')

  const rows = run.jobs.map((job) => {
    const runner = [job.agent, job.model, job.effort].filter(Boolean).join(' / ')
    const tokens = job.tokens?.total?.totalTokens ?? 0
    return `| ${job.id} | ${cell(runner)} | ${job.status}${job.outcome ? ` (${cell(job.outcome)})` : ''} | ${tokens} | ${job.costUsd !== null ? fmtUsd(job.costUsd) : '—'} | ${job.seconds !== null ? Math.round(job.seconds) : '—'} s |`
  })

  const details = run.jobs.map((job) => {
    const lines = [
      `### ${job.id}`,
      '',
      `- Task: ${clip(job.spec, SPEC_LIMIT)}`,
      `- Folder: \`${job.cwd}\``,
    ]
    if (job.worktree) lines.push(`- Worktree: \`${job.worktree}\``)
    if (job.summary.trim()) lines.push(`- Result: ${clip(job.summary, SUMMARY_LIMIT)}`)
    return lines.join('\n')
  })

  const body = [
    '## Workers',
    '',
    '| Worker | Agent / model / effort | Status | Tokens | Cost | Time |',
    '| --- | --- | --- | --- | --- | --- |',
    ...rows,
    '',
    '## Details',
    '',
    details.join('\n\n'),
    '',
    'Recorded from the Coding Agent orchestration board. Costs are what the providers reported.',
  ].join('\n')

  return {
    title: run.label,
    folder: project ? `30-Projetos/${project}` : '00-Inbox',
    summary,
    body,
    tags: project ? ['orquestracao', project] : ['orquestracao'],
  }
}

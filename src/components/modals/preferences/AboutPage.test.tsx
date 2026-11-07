import { fireEvent, render, screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { version as appVersion } from '../../../../package.json'
import { fetchCodingAgentReleases } from '../../../lib/releases'
import { openInBrowser } from '../../../lib/tauri/filesystem'
import { EMPTY_PROJECTS_FILE } from '../../../lib/types'
import { installRelease } from '../../../lib/updater'
import { useProjectsStore } from '../../../stores/projectsStore'
import { AboutPage } from './AboutPage'

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn(async () => '1.7.0'),
  getTauriVersion: vi.fn(async () => '2.8.0'),
}))

vi.mock('../../../lib/tauri/filesystem', () => ({
  openInBrowser: vi.fn(async () => {}),
  openDataFolder: vi.fn(async () => {}),
  openLogsFolder: vi.fn(async () => {}),
}))

vi.mock('../../../lib/backup', () => ({
  exportProfileState: vi.fn(async () => ({ skipped: [], embeddedImages: 0, imageLinksKept: [] })),
  announceBackup: vi.fn(),
}))

vi.mock('../../../lib/dialog', () => ({
  saveFile: vi.fn(async () => 'C:/backups/coding-agent.zip'),
}))

vi.mock('../../../lib/updater', () => ({
  checkForUpdate: vi.fn(async () => null),
  installPendingUpdate: vi.fn(async () => {}),
  installRelease: vi.fn(() => new Promise<void>(() => {})),
}))

vi.mock('../../../lib/releases', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../lib/releases')>()),
  // The newest release is always the running version, so the test survives version bumps.
  fetchCodingAgentReleases: vi.fn(async () => [
    {
      version: (await import('../../../../package.json')).version,
      publishedAt: '2026-09-20T12:00:00Z',
      installable: true,
    },
    { version: '1.6.0', publishedAt: '2026-08-17T12:00:00Z', installable: true },
    { version: '1.2.3', publishedAt: '2026-07-01T12:00:00Z', installable: false },
  ]),
}))

describe('AboutPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useProjectsStore.setState({ ...structuredClone(EMPTY_PROJECTS_FILE), hydrated: true })
  })

  it('credits Alethe and links to the original project', async () => {
    render(<AboutPage />)
    expect(screen.getByText('Based on Alethe')).toBeInTheDocument()
    expect(await screen.findByText('com.codingagent.desktop · Tauri 2.8.0')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Alethe' }))
    expect(openInBrowser).toHaveBeenCalledWith('https://github.com/Kc1t/alethe-agents')
  })

  it('opens the contribution pages on GitHub', () => {
    render(<AboutPage />)
    fireEvent.click(screen.getByRole('button', { name: 'Open a pull request' }))
    fireEvent.click(screen.getByRole('button', { name: 'Report a bug' }))
    expect(openInBrowser).toHaveBeenCalledWith(
      'https://github.com/oigorbrazil/coding-agent/compare',
    )
    expect(openInBrowser).toHaveBeenCalledWith(
      'https://github.com/oigorbrazil/coding-agent/issues/new?template=bug_report.yml',
    )
  })

  it('lists only older releases and asks before installing one', async () => {
    render(<AboutPage />)
    fireEvent.click(screen.getByRole('button', { name: 'Show older versions' }))

    const older = await screen.findByText('v1.6.0')
    expect(fetchCodingAgentReleases).toHaveBeenCalledTimes(1)
    const list = older.closest('ul') as HTMLElement
    expect(within(list).queryByText(`v${appVersion}`)).not.toBeInTheDocument()
    const row = older.closest('li') as HTMLElement

    fireEvent.click(within(row).getByRole('button', { name: 'Install' }))
    expect(within(row).getByText(/export a backup first/)).toBeInTheDocument()
    expect(installRelease).not.toHaveBeenCalled()

    fireEvent.click(within(row).getByRole('button', { name: 'Install v1.6.0' }))
    expect(installRelease).toHaveBeenCalledWith('1.6.0', expect.any(Function))
    expect(await within(row).findByText('Downloading v1.6.0… 0%')).toBeInTheDocument()
  })

  it('sends releases the updater cannot install to their GitHub page', async () => {
    render(<AboutPage />)
    fireEvent.click(screen.getByRole('button', { name: 'Show older versions' }))
    const row = (await screen.findByText('v1.2.3')).closest('li') as HTMLElement
    fireEvent.click(within(row).getByRole('button', { name: 'Download' }))
    expect(openInBrowser).toHaveBeenCalledWith(
      'https://github.com/oigorbrazil/coding-agent/releases/tag/v1.2.3',
    )
    expect(installRelease).not.toHaveBeenCalled()
  })
})

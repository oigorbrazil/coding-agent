import { useCallback, useEffect, useRef, useState } from 'react'

import { useT } from '../../../lib/i18n'
import { openInFileExplorer, type SecondBrainStatus, secondBrainStatus } from '../../../lib/tauri'
import controls from '../controls.module.css'
import { SettingsSection } from './primitives'
import styles from './SecondBrainPanel.module.css'

export function SecondBrainPanel() {
  const t = useT()
  const [status, setStatus] = useState<SecondBrainStatus | null>(null)
  const [failed, setFailed] = useState(false)
  const mountedRef = useRef(true)

  const refresh = useCallback(async () => {
    try {
      const next = await secondBrainStatus()
      if (!mountedRef.current) return
      setStatus(next)
      setFailed(false)
    } catch {
      if (mountedRef.current) setFailed(true)
    }
  }, [])

  useEffect(() => {
    mountedRef.current = true
    void refresh()
    return () => {
      mountedRef.current = false
    }
  }, [refresh])

  return (
    <SettingsSection
      id="second-brain"
      title={t('secondBrain.title')}
      description={t('secondBrain.description')}
    >
      <div className={styles.panel}>
        <p className={styles.state}>
          {failed
            ? t('secondBrain.error')
            : status === null
              ? t('secondBrain.checking')
              : status.exists
                ? t('secondBrain.found', { count: status.noteCount, path: status.path })
                : t('secondBrain.missing', { path: status.path })}
        </p>
        {status?.fromEnv ? <p className={styles.state}>{t('secondBrain.fromEnv')}</p> : null}

        {status?.exists && status.recent.length > 0 ? (
          <div className={styles.recent}>
            <span className={styles.recentTitle}>{t('secondBrain.recent')}</span>
            <ul className={styles.list}>
              {status.recent.map((note) => (
                <li key={note.path} className={styles.note} title={note.path}>
                  <span className={styles.noteTitle}>{note.title}</span>
                  <span className={styles.notePath}>{note.path}</span>
                </li>
              ))}
            </ul>
          </div>
        ) : null}

        <div className={styles.actions}>
          {status?.exists ? (
            <button
              type="button"
              className={controls.btn}
              onClick={() => void openInFileExplorer(status.path)}
            >
              {t('secondBrain.open')}
            </button>
          ) : null}
          <button type="button" className={controls.btn} onClick={() => void refresh()}>
            {t('secondBrain.refresh')}
          </button>
        </div>
      </div>
    </SettingsSection>
  )
}

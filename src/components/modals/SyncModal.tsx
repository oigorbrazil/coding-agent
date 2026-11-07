import { useT } from '../../lib/i18n'
import { useUiStore } from '../../stores/uiStore'
import { Modal } from './Modal'
import { CloudCard } from './sync/CloudCard'
import { FileCard } from './sync/FileCard'
import { LegacyGistCard } from './sync/LegacyGistCard'
import styles from './SyncModal.module.css'

export function SyncModal() {
  const t = useT()
  const open = useUiStore((s) => s.openModal === 'sync')
  const closeModal = useUiStore((s) => s.closeModal)

  return (
    <Modal open={open} onClose={closeModal} title={t('sync.title')} width={620}>
      <p className={styles.subtitle}>{t('sync.subtitle')}</p>

      <div className={styles.grid}>
        <CloudCard open={open} />
        <FileCard open={open} />
        <LegacyGistCard open={open} />
      </div>
    </Modal>
  )
}

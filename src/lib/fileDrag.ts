export const CODING_AGENT_FILE_DRAG_TYPE = 'application/x-coding-agent-file'

export type CodingAgentFileDragPayload = {
  projectId: string
  path: string
}

export function writeFileDragPayload(
  dataTransfer: DataTransfer,
  payload: CodingAgentFileDragPayload,
): void {
  dataTransfer.effectAllowed = 'copy'
  dataTransfer.setData(CODING_AGENT_FILE_DRAG_TYPE, JSON.stringify(payload))
  dataTransfer.setData('text/plain', payload.path)
}

export function readFileDragPayload(dataTransfer: DataTransfer): CodingAgentFileDragPayload | null {
  const raw = dataTransfer.getData(CODING_AGENT_FILE_DRAG_TYPE)
  if (!raw) return null
  try {
    const value = JSON.parse(raw) as Partial<CodingAgentFileDragPayload>
    if (typeof value.projectId !== 'string' || typeof value.path !== 'string') return null
    const projectId = value.projectId.trim()
    const path = value.path.trim()
    return projectId && path ? { projectId, path } : null
  } catch {
    return null
  }
}

export function hasFileDragPayload(dataTransfer: DataTransfer): boolean {
  return Array.from(dataTransfer.types).includes(CODING_AGENT_FILE_DRAG_TYPE)
}

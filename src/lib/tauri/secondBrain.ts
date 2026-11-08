import { invoke } from '@tauri-apps/api/core'

export type SecondBrainNote = {
  /** Relative to the second brain root, with forward slashes. */
  path: string
  title: string
  modifiedMs: number
}

export type SecondBrainStatus = {
  path: string
  exists: boolean
  /** `CODING_OPUS_BRAIN` chose the folder instead of the Documents default. */
  fromEnv: boolean
  noteCount: number
  folders: string[]
  recent: SecondBrainNote[]
}

export type SecondBrainRecordRequest = {
  title: string
  /** Relative to the root; defaults to 00-Inbox. */
  folder?: string
  summary?: string
  body?: string
  tags?: string[]
  /** `nota` by default; `decisao` adds the Motivo and Consequências sections. */
  kind?: 'decisao' | 'nota' | 'estudo' | 'ideia' | 'meeting'
  area?: string
  /** Wikilink targets listed under "Notas relacionadas". */
  related?: string[]
  /** Local time as YYYYMMDD-HHMM: the note id and dates follow the user's clock, as in the skill. */
  localStamp?: string
}

export async function secondBrainStatus(): Promise<SecondBrainStatus> {
  return invoke<SecondBrainStatus>('second_brain_status')
}

/** Writes a new note and returns its absolute path; never replaces an existing file. */
export async function secondBrainRecord(request: SecondBrainRecordRequest): Promise<string> {
  return invoke<string>('second_brain_record', { request })
}

import { defineMessages } from '@modrinth/ui'

export type ShareMethod = 'direct' | 'link'
export type MethodFilter = ShareMethod | 'all'
export type ShareTableColumn = 'username' | 'lastPlayed' | 'joined' | 'method' | 'actions'

export const SHARED_INSTANCE_USER_LIMIT = 50

export type ShareRow = {
	id: string
	username: string
	avatarUrl?: string
	lastPlayedAt: Date | null
	joinedAt: Date | null
	method: ShareMethod
	pending?: boolean
}

/** How a member got in, as shown in the members table and the remove dialog. */
export const methodMessages = defineMessages({
	direct: { id: 'app.instance.share.method.direct', defaultMessage: 'Direct invite' },
	link: { id: 'app.instance.share.method.link', defaultMessage: 'Share link' },
}) as Record<ShareMethod, { id: string; defaultMessage: string }>

export { normalizeInviteKey } from '@modrinth/ui'

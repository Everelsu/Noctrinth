/**
 * Query language for the instance grid search field.
 *
 * Bare words match the instance name, so anything typed before this existed keeps
 * working. Sigils opt into the richer filters:
 *
 *   sodium              instance name contains "sodium"
 *   @sodium             instance has content whose name contains "sodium"
 *   #shader             instance has content of that type
 *   !outdated           state filter (outdated / disabled / broken)
 *   -@sodium            negation — instance has no such content
 *   @"fabric api"       quotes for values with spaces
 *
 * Terms combine with AND. The content-level terms (`@`, `#`, `!outdated`,
 * `!disabled`) are evaluated against a *single* content item rather than the
 * instance as a whole, so `@sodium !outdated` means "Sodium itself is out of
 * date here", not "Sodium is installed and something unrelated is stale".
 */
import type { ContentItem } from '@modrinth/ui'

import type { GameInstance } from './types'

export type ContentTypeFilter = 'mod' | 'datapack' | 'resourcepack' | 'shaderpack'

export type TokenKind = 'name' | 'content' | 'type' | 'state'

export interface SearchToken {
	kind: TokenKind
	/** Normalised value: lowercased, and mapped to a canonical name for `type`/`state`. */
	value: string
	negated: boolean
	/** The token exactly as typed, for error messages. */
	raw: string
	/** True when the sigil is known but the value is not in its vocabulary. */
	unknown: boolean
}

export interface ParsedQuery {
	tokens: SearchToken[]
	/** Terms that need the per-instance content list to be resolved. */
	needsContent: boolean
	/** Tokens whose value isn't a recognised type/state, surfaced as a hint. */
	unknown: SearchToken[]
	isEmpty: boolean
}

export interface InstanceMatch {
	instance: GameInstance
	/** Content items that satisfied the positive content terms, for display on the card. */
	contentMatches: ContentItem[]
}

const TYPE_ALIASES: Record<string, ContentTypeFilter> = {
	mod: 'mod',
	mods: 'mod',
	shader: 'shaderpack',
	shaders: 'shaderpack',
	shaderpack: 'shaderpack',
	shaderpacks: 'shaderpack',
	resourcepack: 'resourcepack',
	resourcepacks: 'resourcepack',
	resource: 'resourcepack',
	texturepack: 'resourcepack',
	rp: 'resourcepack',
	datapack: 'datapack',
	datapacks: 'datapack',
	data: 'datapack',
	dp: 'datapack',
}

const STATE_ALIASES: Record<string, string> = {
	outdated: 'outdated',
	update: 'outdated',
	updatable: 'outdated',
	stale: 'outdated',
	disabled: 'disabled',
	off: 'disabled',
	broken: 'broken',
}

/** States that describe a single piece of content rather than the instance. */
const CONTENT_STATES = new Set(['outdated', 'disabled'])

export const KNOWN_TYPES = Object.keys(TYPE_ALIASES)
export const KNOWN_STATES = Object.keys(STATE_ALIASES)

/** A completion offered under the search field. */
export interface SearchSuggestion {
	/** Replaces the token being typed. */
	insert: string
	label: string
	hint: string
	/** `sigil` entries only prime the next keystroke, so the field stays open. */
	kind: 'sigil' | 'content' | 'type' | 'state'
	iconUrl?: string | null
}

const SIGIL_SUGGESTIONS: SearchSuggestion[] = [
	{
		insert: '@',
		label: '@ — by mod',
		hint: 'Instances that have a given mod, resource pack or shader',
		kind: 'sigil',
	},
	{
		insert: '#',
		label: '# — by type',
		hint: 'Instances that have any mod, shader, resource pack or data pack',
		kind: 'sigil',
	},
	{
		insert: '!',
		label: '! — by state',
		hint: 'Instances with something outdated, disabled or broken',
		kind: 'sigil',
	},
]

const TYPE_SUGGESTIONS: SearchSuggestion[] = [
	{ insert: '#mod', label: '#mod', hint: 'Has mods', kind: 'type' },
	{ insert: '#shader', label: '#shader', hint: 'Has shader packs', kind: 'type' },
	{
		insert: '#resourcepack',
		label: '#resourcepack',
		hint: 'Has resource packs',
		kind: 'type',
	},
	{ insert: '#datapack', label: '#datapack', hint: 'Has data packs', kind: 'type' },
]

const STATE_SUGGESTIONS: SearchSuggestion[] = [
	{
		insert: '!outdated',
		label: '!outdated',
		hint: 'Something installed has an update',
		kind: 'state',
	},
	{
		insert: '!disabled',
		label: '!disabled',
		hint: 'Something is installed but switched off',
		kind: 'state',
	},
	{
		insert: '!broken',
		label: '!broken',
		hint: 'The instance itself needs repairing',
		kind: 'state',
	},
]

/** The whitespace-separated chunk the user is currently typing. */
export function activeToken(input: string): string {
	if (/\s$/.test(input) || !input) return ''
	return input.slice(input.lastIndexOf(' ') + 1)
}

/** Replaces the chunk being typed, leaving the earlier terms untouched. */
export function applySuggestion(input: string, suggestion: SearchSuggestion): string {
	const head = input.slice(0, input.length - activeToken(input).length)
	// A bare sigil is only half a term, so don't close it off with a space yet.
	return suggestion.kind === 'sigil'
		? `${head}${suggestion.insert}`
		: `${head}${suggestion.insert} `
}

/** True when a value needs quoting to survive the tokenizer. */
function quoteIfNeeded(value: string): string {
	return /\s/.test(value) ? `"${value}"` : value
}

/**
 * Completions for whatever is being typed.
 *
 * `contentNames` come from the loaded content index, so `@` completions only
 * appear once that has streamed in — which is also when they're useful.
 */
export function suggestionsFor(
	input: string,
	contentNames: { name: string; iconUrl?: string | null }[],
	limit = 8,
): SearchSuggestion[] {
	const token = activeToken(input)
	const negated = token.startsWith('-')
	const bare = negated ? token.slice(1) : token
	const prefix = negated ? '-' : ''

	const withPrefix = (suggestions: SearchSuggestion[]) =>
		prefix
			? suggestions.map((suggestion) => ({
					...suggestion,
					insert: `${prefix}${suggestion.insert}`,
				}))
			: suggestions

	if (!bare) return withPrefix(SIGIL_SUGGESTIONS)

	const sigil = bare[0]
	const value = bare.slice(1).toLowerCase()

	if (sigil === '#') {
		return withPrefix(
			TYPE_SUGGESTIONS.filter((suggestion) => suggestion.insert.slice(1).startsWith(value)),
		).slice(0, limit)
	}

	if (sigil === '!') {
		return withPrefix(
			STATE_SUGGESTIONS.filter((suggestion) => suggestion.insert.slice(1).startsWith(value)),
		).slice(0, limit)
	}

	if (sigil === '@') {
		const seen = new Set<string>()
		const matches: SearchSuggestion[] = []

		for (const entry of contentNames) {
			const lower = entry.name.toLowerCase()
			if (!lower.includes(value) || seen.has(lower)) continue
			seen.add(lower)
			matches.push({
				insert: `${prefix}@${quoteIfNeeded(entry.name)}`,
				label: entry.name,
				hint: 'Installed in one or more instances',
				kind: 'content',
				iconUrl: entry.iconUrl,
			})
			if (matches.length >= limit) break
		}

		return matches
	}

	// A bare word searches instance names; nothing useful to complete.
	return []
}

/** Splits on whitespace, but keeps double-quoted runs together. */
function tokenize(input: string): string[] {
	const out: string[] = []
	let current = ''
	let inQuotes = false

	for (const char of input) {
		if (char === '"') {
			inQuotes = !inQuotes
			continue
		}
		if (!inQuotes && /\s/.test(char)) {
			if (current) out.push(current)
			current = ''
			continue
		}
		current += char
	}

	if (current) out.push(current)
	return out
}

function parseToken(raw: string): SearchToken | null {
	let rest = raw
	let negated = false

	if (rest.length > 1 && rest.startsWith('-')) {
		negated = true
		rest = rest.slice(1)
	}

	const sigil = rest[0]
	if (sigil === '@' || sigil === '#' || sigil === '!') {
		const value = rest.slice(1).toLowerCase()
		// A lone sigil means the user is still typing — ignore rather than match nothing.
		if (!value) return null

		if (sigil === '@') {
			return { kind: 'content', value, negated, raw, unknown: false }
		}
		if (sigil === '#') {
			const canonical = TYPE_ALIASES[value]
			return { kind: 'type', value: canonical ?? value, negated, raw, unknown: !canonical }
		}
		const canonical = STATE_ALIASES[value]
		return { kind: 'state', value: canonical ?? value, negated, raw, unknown: !canonical }
	}

	const value = rest.toLowerCase()
	if (!value) return null
	return { kind: 'name', value, negated, raw, unknown: false }
}

function tokenNeedsContent(token: SearchToken): boolean {
	if (token.unknown) return false
	if (token.kind === 'content' || token.kind === 'type') return true
	return token.kind === 'state' && CONTENT_STATES.has(token.value)
}

export function parseQuery(input: string): ParsedQuery {
	const tokens = tokenize(input)
		.map(parseToken)
		.filter((token): token is SearchToken => token !== null)

	return {
		tokens,
		needsContent: tokens.some(tokenNeedsContent),
		unknown: tokens.filter((token) => token.unknown),
		isEmpty: tokens.length === 0,
	}
}

function contentName(item: ContentItem): string {
	return (item.project?.title ?? item.embedded_metadata?.name ?? item.file_name ?? '').toLowerCase()
}

/** Whether a single content item satisfies a content-level term, ignoring negation. */
function contentTokenMatches(token: SearchToken, item: ContentItem): boolean {
	switch (token.kind) {
		case 'content':
			return (
				contentName(item).includes(token.value) ||
				(item.project?.slug?.toLowerCase().includes(token.value) ?? false)
			)
		case 'type':
			return item.project_type === token.value
		case 'state':
			if (token.value === 'outdated') return item.has_update === true
			if (token.value === 'disabled') return item.enabled === false
			return false
		default:
			return false
	}
}

function isInstanceBroken(instance: GameInstance): boolean {
	return instance.quarantined || instance.install_stage !== 'installed'
}

/**
 * Applies a parsed query to one instance.
 *
 * `content` may be undefined when the index hasn't loaded yet; in that case
 * content-level terms are skipped so the grid still narrows by the terms it can
 * already answer. Callers should show that the result is provisional.
 */
export function matchInstance(
	instance: GameInstance,
	query: ParsedQuery,
	content: ContentItem[] | undefined,
): InstanceMatch | null {
	const name = instance.name.toLowerCase()

	const positiveContent: SearchToken[] = []
	const negativeContent: SearchToken[] = []

	for (const token of query.tokens) {
		// An unrecognised #foo / !foo can't match anything, so it filters everything
		// out. That's noisier than it is helpful — treat it as inert and let the UI
		// tell the user the term was not understood.
		if (token.unknown) continue

		if (token.kind === 'name') {
			const hit = name.includes(token.value)
			if (hit === token.negated) return null
			continue
		}

		if (token.kind === 'state' && !CONTENT_STATES.has(token.value)) {
			if (token.value === 'broken') {
				const hit = isInstanceBroken(instance)
				if (hit === token.negated) return null
			}
			continue
		}

		if (token.negated) negativeContent.push(token)
		else positiveContent.push(token)
	}

	if (!positiveContent.length && !negativeContent.length) {
		return { instance, contentMatches: [] }
	}

	// Index still loading: keep the instance, but claim no content matches.
	if (!content) return { instance, contentMatches: [] }

	let contentMatches: ContentItem[] = []
	if (positiveContent.length) {
		contentMatches = content.filter((item) =>
			positiveContent.every((token) => contentTokenMatches(token, item)),
		)
		if (!contentMatches.length) return null
	}

	if (
		negativeContent.length &&
		content.some((item) => negativeContent.every((token) => contentTokenMatches(token, item)))
	) {
		return null
	}

	return { instance, contentMatches }
}

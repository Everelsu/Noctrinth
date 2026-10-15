// The small Markdown the site's pages are written in: #/##/### headings,
// - bullets, > quotes, **bold**, *italic*, `code`, [text](url) links and
// ![alt](src) images. Kept dependency-free: the Pages workflow runs this with
// a bare Node and no install.

export function escapeHtml(s) {
	return s
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
}

/** One line of inline Markdown. `resolveImage` turns an image path into one the page can load. */
export function renderInline(s, resolveImage = (src) => src) {
	return (
		escapeHtml(s)
			.replace(/`([^`]+)`/g, '<code>$1</code>')
			.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
			.replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>')
			// Images before links: the syntax differs by one leading `!`.
			.replace(
				/!\[([^\]]*)\]\(([^)]+)\)/g,
				(_, alt, src) => `<img src="${resolveImage(src)}" alt="${alt}" loading="lazy">`,
			)
			.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" rel="noopener">$1</a>')
	)
}

export function renderMarkdown(src, resolveImage) {
	const lines = src.replace(/\r\n/g, '\n').split('\n')
	const out = []
	let i = 0
	const inline = (s) => renderInline(s, resolveImage)

	while (i < lines.length) {
		const trimmed = lines[i].trim()

		if (!trimmed) {
			i++
			continue
		}

		const heading = trimmed.match(/^(#{1,4}) (.*)$/)
		if (heading) {
			const level = heading[1].length
			out.push(`<h${level}>${inline(heading[2].trim())}</h${level}>`)
			i++
			continue
		}

		if (trimmed.startsWith('- ')) {
			const items = []
			while (i < lines.length && lines[i].trim().startsWith('- ')) {
				items.push(lines[i].trim().slice(2).trim())
				i++
			}
			out.push(`<ul>${items.map((it) => `<li>${inline(it)}</li>`).join('')}</ul>`)
			continue
		}

		if (trimmed.startsWith('> ')) {
			const quote = []
			while (i < lines.length && lines[i].trim().startsWith('> ')) {
				quote.push(lines[i].trim().slice(2))
				i++
			}
			out.push(`<blockquote><p>${inline(quote.join(' '))}</p></blockquote>`)
			continue
		}

		// An image alone on its line is a figure rather than part of a paragraph.
		if (/^!\[[^\]]*\]\([^)]+\)$/.test(trimmed)) {
			out.push(`<figure>${inline(trimmed)}</figure>`)
			i++
			continue
		}

		const paraLines = []
		while (i < lines.length) {
			const t = lines[i].trim()
			if (!t || t.startsWith('#') || t.startsWith('- ') || t.startsWith('> ')) break
			paraLines.push(t)
			i++
		}
		out.push(`<p>${inline(paraLines.join(' '))}</p>`)
	}

	return out.join('\n')
}

/**
 * The `---` block at the top of a Markdown file, and what follows it.
 *
 * Only `key: value` lines, which is all a news article needs; a value may be
 * wrapped in single or double quotes.
 */
export function parseFrontmatter(src) {
	const match = src.replace(/\r\n/g, '\n').match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/)
	if (!match) return { data: {}, content: src }

	const data = {}
	for (const line of match[1].split('\n')) {
		const pair = line.match(/^([A-Za-z_][\w-]*):\s*(.*)$/)
		if (!pair) continue
		data[pair[1]] = pair[2].trim().replace(/^(['"])(.*)\1$/, '$2')
	}
	return { data, content: match[2] }
}

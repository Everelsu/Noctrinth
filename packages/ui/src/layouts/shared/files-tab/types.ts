export interface FileItem {
	name: string
	type: 'file' | 'directory' | 'symlink'
	path: string
	modified: number
	created: number
	size?: number | null
	count?: number | null
	target?: string
	readOnly?: boolean
}

/**
 * What a file is, where the host knows: an installed project's mod or pack,
 * drawn with that project's icon, name, version and author.
 */
export interface FileItemDescription {
	/** Without one, the row keeps its file name and only takes the picture. */
	title?: string
	iconUrl?: string | null
	version?: string | null
	author?: { name: string; open?: () => void } | null
	/** Opens the project's page. */
	open?: () => void
}

export interface EditingFile {
	name: string
	path: string
}

export type FileSortField = 'name' | 'size' | 'created' | 'modified'

export type FileViewFilter = 'all' | 'filesOnly' | 'foldersOnly'

export interface FileOperation {
	id?: string
	op: string
	src: string
	state: string
	progress?: number
	bytes_processed?: number
	files_processed?: number
	current_file?: string
	cancellable?: boolean
	error?: string
}

export interface UndoableOperation {
	type: 'move' | 'rename'
	itemType: string
	fileName: string
}

export interface MoveOperation extends UndoableOperation {
	type: 'move'
	sourcePath: string
	destinationPath: string
}

export interface RenameOperation extends UndoableOperation {
	type: 'rename'
	path: string
	oldName: string
	newName: string
}

export type Operation = MoveOperation | RenameOperation

export interface ExtractDryRunResult {
	modpack_name: string | null
	conflicting_files: string[]
}

export type { UploadState } from '@modrinth/api-client'

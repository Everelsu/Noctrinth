import { invoke } from '@tauri-apps/api/core'

export type ElyCredentials = {
  profile: {
    id: string
    name: string
    skins: unknown[]
    capes: unknown[]
  }
  access_token: string
  active: boolean
  auth_provider: 'ely_by'
}

export async function ely_login(username: string, password: string): Promise<ElyCredentials> {
  return await invoke('plugin:ely-auth|ely_login', { username, password })
}

export async function ely_logout(user: string): Promise<void> {
  await invoke('plugin:ely-auth|ely_logout', { user })
}

export async function ely_get_users(): Promise<ElyCredentials[]> {
  return await invoke('plugin:ely-auth|ely_get_users')
}

export async function ely_get_default_user(): Promise<string | null> {
  return await invoke('plugin:ely-auth|ely_get_default_user')
}

export async function ely_set_default_user(user: string): Promise<void> {
  await invoke('plugin:ely-auth|ely_set_default_user', { user })
}

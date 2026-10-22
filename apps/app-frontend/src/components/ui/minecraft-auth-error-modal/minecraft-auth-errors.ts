import { defineMessage } from '@modrinth/ui'

/**
 * A sign-in failure we recognise, with what to tell the player about it.
 *
 * The advice is held as message descriptors rather than plain strings: this
 * modal is what a player reads when they cannot get into the game, which is
 * the worst possible moment to hand them a language they do not speak. Links
 * live beside the message rather than inside it, because ICU has no room for
 * an `href` and a translator has no business retyping one.
 */
export interface AuthErrorMessage {
	id: string
	defaultMessage: string
}

export interface AuthErrorStep {
	message: AuthErrorMessage
	/** Where this step's `<link>` points, when it has one. */
	href?: string
}

export interface MinecraftAuthError {
	whatHappened: AuthErrorMessage
	stepsToFix: AuthErrorStep[]
}

const networkError: MinecraftAuthError = {
	whatHappened: defineMessage({
		id: 'app.minecraft-auth-error.network.what',
		defaultMessage:
			'Noctrinth could not connect to a Microsoft, Xbox, or Minecraft service needed for sign-in. This is usually caused by a local network, DNS, proxy, firewall, hosts file, VPN, or antivirus issue.',
	}),
	stepsToFix: [
		{
			message: defineMessage({
				id: 'app.minecraft-auth-error.network.step-1',
				defaultMessage: 'Restart Noctrinth and try signing in again',
			}),
		},
		{
			message: defineMessage({
				id: 'app.minecraft-auth-error.network.step-2',
				defaultMessage: 'Check that your internet connection is working',
			}),
		},
		{
			message: defineMessage({
				id: 'app.minecraft-auth-error.network.step-3',
				defaultMessage:
					'Allow Noctrinth through your firewall, antivirus, proxy, VPN, and hosts file rules',
			}),
		},
		{
			message: defineMessage({
				id: 'app.minecraft-auth-error.network.step-4',
				defaultMessage:
					'Try a different network or temporarily disable VPN/proxy software if you use one',
			}),
		},
		{
			message: defineMessage({
				id: 'app.minecraft-auth-error.network.step-5',
				defaultMessage:
					'If routing or DNS is the issue, a service like Cloudflare WARP can sometimes help',
			}),
		},
	],
}

export const minecraftAuthErrors: Record<string, MinecraftAuthError> = {
	minecraft_auth_token_expired: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.refresh-token.what',
			defaultMessage:
				'Your saved Microsoft sign-in token has expired or was revoked, so Noctrinth cannot refresh your Minecraft session.',
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.refresh-token.step-1',
					defaultMessage: 'Sign out of the affected Minecraft account in Noctrinth',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.refresh-token.step-2',
					defaultMessage: 'Sign in to the account again',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.refresh-token.step-3',
					defaultMessage: 'Once the new sign-in finishes, try launching Minecraft again',
				}),
			},
		],
	},
	minecraft_auth_clock_out_of_sync: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.sisu-authenticate.what',
			defaultMessage:
				'Xbox services rejected the first sign-in response. This is most often caused by your system clock or time zone being out of sync.',
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.sisu-authenticate.step-1',
					defaultMessage: 'Open your system date and time settings',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.sisu-authenticate.step-2',
					defaultMessage: 'Turn on automatic time zone and automatic time, if available',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.sisu-authenticate.step-3',
					defaultMessage: 'Use the sync option in your system settings to synchronize the clock',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.sisu-authenticate.step-4',
					defaultMessage: 'Restart Noctrinth',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.sisu-authenticate.step-5',
					defaultMessage: 'Try signing in again',
				}),
			},
		],
	},
	minecraft_auth_rate_limited: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.too-many-requests.what',
			defaultMessage:
				'Microsoft or Minecraft temporarily blocked the sign-in request because there were too many recent attempts.',
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.too-many-requests.step-1',
					defaultMessage: 'Wait about an hour before trying again',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.too-many-requests.step-2',
					defaultMessage: 'Restart Noctrinth after waiting',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.too-many-requests.step-3',
					defaultMessage: 'Try signing in once more',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.too-many-requests.step-4',
					defaultMessage:
						'If the same message appears, wait longer before retrying so the temporary limit can clear',
				}),
			},
		],
	},
	minecraft_auth_service_unavailable: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.minecraft-server-error.what',
			defaultMessage:
				"Minecraft's authentication service is returning a server error, so Noctrinth cannot finish signing you in right now.",
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.minecraft-server-error.step-1',
					defaultMessage: 'Wait a few minutes and try signing in again',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.minecraft-server-error.step-2',
					defaultMessage: 'Check <link>Xbox Status</link> for current service issues',
				}),
				href: 'https://support.xbox.com/xbox-live-status',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.minecraft-server-error.step-3',
					defaultMessage:
						'Try signing in with the <link>official Minecraft Launcher</link> to confirm whether Minecraft sign-in is also affected there',
				}),
				href: 'https://www.minecraft.net/en-us/download',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.minecraft-server-error.step-4',
					defaultMessage:
						'If the service is healthy and this keeps happening, contact support with the debug information below',
				}),
			},
		],
	},
	minecraft_auth_no_profile: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.player-profile.what',
			defaultMessage:
				'Minecraft services could not return a Java Edition profile for this account. This most often happens when the game was purchased recently, the Java profile has not finished being created, or the wrong Microsoft account is being used.',
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.player-profile.step-1',
					defaultMessage: 'Sign in with the <link>official Minecraft Launcher</link>',
				}),
				href: 'https://www.minecraft.net/en-us/download',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.player-profile.step-2',
					defaultMessage: 'Launch Minecraft: Java Edition once from the official launcher',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.player-profile.step-3',
					defaultMessage: 'Wait up to an hour if the purchase or profile setup was recent',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.player-profile.step-4',
					defaultMessage:
						'Make sure you are using the Microsoft account that owns Minecraft. See <link>Finding the right Xbox account</link> for help',
				}),
				href: 'https://support.modrinth.com/en/articles/9409136-finding-the-right-xbox-account',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.player-profile.step-5',
					defaultMessage: 'Try signing in to Noctrinth again',
				}),
			},
		],
	},
	minecraft_auth_network: networkError,
	minecraft_auth_connection_refused: networkError,
	minecraft_auth_age_verification: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.uk-age-verification.what',
			defaultMessage:
				'Your Minecraft/Xbox Live account requires age verification to comply with UK regulations. You must complete this before signing in.',
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.uk-age-verification.step-1',
					defaultMessage: 'Go to the <link>Minecraft Login</link> page and sign in',
				}),
				href: 'https://www.minecraft.net/en-us/login',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.uk-age-verification.step-2',
					defaultMessage: 'Follow the instructions to verify your age',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.uk-age-verification.step-3',
					defaultMessage: 'Once verified, try signing in again',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.uk-age-verification.step-4',
					defaultMessage: 'For additional help, visit <link>UK age verification on Xbox</link>',
				}),
				href: 'https://support.xbox.com/en-GB/help/family-online-safety/online-safety/UK-age-verification',
			},
		],
	},
	minecraft_auth_no_xbox_profile: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.no-xbox-profile.what',
			defaultMessage: "This account doesn't have an Xbox profile set up or doesn't own Minecraft.",
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.no-xbox-profile.step-1',
					defaultMessage: 'Make sure Minecraft is purchased on this account',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.no-xbox-profile.step-2',
					defaultMessage: 'Visit <link>Minecraft Login</link> and sign in',
				}),
				href: 'https://www.minecraft.net/en-us/login',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.no-xbox-profile.step-3',
					defaultMessage: 'Complete Xbox profile setup if prompted',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.no-xbox-profile.step-4',
					defaultMessage: 'Once finished, try signing in again',
				}),
			},
		],
	},
	minecraft_auth_region_unavailable: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.region-unavailable.what',
			defaultMessage: "Xbox Live isn't available in your region, so sign-in is blocked.",
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.region-unavailable.step-1',
					defaultMessage: 'Xbox services must be supported in your country before you can sign in',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.region-unavailable.step-2',
					defaultMessage: 'Check <link>Xbox Availability</link> for supported regions',
				}),
				href: 'https://www.xbox.com/en-US/regions',
			},
		],
	},
	minecraft_auth_adult_verification: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.korea-adult-verification.what',
			defaultMessage: 'This account requires adult verification under South Korean regulations.',
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.korea-adult-verification.step-1',
					defaultMessage: 'Visit <link>Xbox</link> and sign in',
				}),
				href: 'https://www.xbox.com',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.korea-adult-verification.step-2',
					defaultMessage: 'Complete the identity verification process',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.korea-adult-verification.step-3',
					defaultMessage: 'Once finished, try signing in again',
				}),
			},
		],
	},
	minecraft_auth_underage: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.underage-no-family.what',
			defaultMessage: 'This account is underage and not linked to a Microsoft family group.',
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.underage-no-family.step-1',
					defaultMessage: 'Review the <link>Family Setup Guide</link>',
				}),
				href: 'https://help.minecraft.net/hc/en-us/articles/4408968616077',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.underage-no-family.step-2',
					defaultMessage: 'Join or create a family group as instructed',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.underage-no-family.step-3',
					defaultMessage: 'Once finished, try signing in again',
				}),
			},
		],
	},
	minecraft_auth_banned: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.suspended.what',
			defaultMessage: 'This account was suspended for violating Xbox Community Standards.',
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.suspended.step-1',
					defaultMessage: 'Visit <link>Xbox Support</link> and review the enforcement details',
				}),
				href: 'https://support.xbox.com',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.suspended.step-2',
					defaultMessage: 'Submit an appeal if one is available',
				}),
			},
		],
	},
	minecraft_auth_restricted: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.restricted-online-play.what',
			defaultMessage: "This account is restricted and doesn't have permission to play online.",
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.restricted-online-play.step-1',
					defaultMessage: 'Have a guardian sign in to <link>Microsoft Family</link>',
				}),
				href: 'https://account.microsoft.com/family/',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.restricted-online-play.step-2',
					defaultMessage: 'Update online play permissions',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.restricted-online-play.step-3',
					defaultMessage: 'Once finished, try signing in again',
				}),
			},
		],
	},
	minecraft_auth_terms_not_accepted: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.terms-not-accepted.what',
			defaultMessage: "This account hasn't accepted Xbox's Terms of Service.",
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.terms-not-accepted.step-1',
					defaultMessage: 'Visit <link>Xbox</link> and sign in',
				}),
				href: 'https://www.xbox.com',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.terms-not-accepted.step-2',
					defaultMessage: 'Accept the Terms if prompted',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.terms-not-accepted.step-3',
					defaultMessage: 'Once finished, try signing in again',
				}),
			},
		],
	},
	minecraft_auth_xsts_rejected: {
		whatHappened: defineMessage({
			id: 'app.minecraft-auth-error.xsts-authorize.what',
			defaultMessage:
				'Xbox services rejected the request to authorize this account for Minecraft services, but did not return a specific account restriction that Noctrinth recognizes.',
		}),
		stepsToFix: [
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.xsts-authorize.step-1',
					defaultMessage: 'Sign in with the <link>official Minecraft Launcher</link>',
				}),
				href: 'https://www.minecraft.net/en-us/download',
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.xsts-authorize.step-2',
					defaultMessage: 'Complete any prompts shown by Microsoft, Xbox, or Minecraft',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.xsts-authorize.step-3',
					defaultMessage: 'Try signing in to Noctrinth again',
				}),
			},
			{
				message: defineMessage({
					id: 'app.minecraft-auth-error.xsts-authorize.step-4',
					defaultMessage:
						'If the official launcher also fails, follow the error shown there or contact Xbox Support',
				}),
			},
		],
	},
}

package com.mojang.authlib.minecraft;

import com.mojang.authlib.GameProfile;

/**
 * The 1.20.2+ arrangement: {@code getTextures} is a default method on the interface, and the
 * Yggdrasil service below it only knows how to unpack what the server sent.
 */
public interface MinecraftSessionService {
    MinecraftProfileTextures unpackTextures(GameProfile profile);

    default MinecraftProfileTextures getTextures(GameProfile profile) {
        return profile.hasSignedTextures() ? unpackTextures(profile) : MinecraftProfileTextures.EMPTY;
    }
}

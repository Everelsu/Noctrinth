package com.mojang.authlib.yggdrasil;

import com.mojang.authlib.GameProfile;
import com.mojang.authlib.minecraft.MinecraftProfileTexture;
import com.mojang.authlib.minecraft.MinecraftProfileTextures;
import com.mojang.authlib.minecraft.MinecraftSessionService;

/** An implementation of the interface the default method lives on, as authlib's own is. */
public class YggdrasilSessionService implements MinecraftSessionService {
    @Override
    public MinecraftProfileTextures unpackTextures(GameProfile profile) {
        return new MinecraftProfileTextures(
                new MinecraftProfileTexture(YggdrasilMinecraftSessionService.SERVER_SKIN, null),
                null,
                null,
                MinecraftProfileTextures.SignatureState.SIGNED);
    }
}

package com.mojang.authlib.yggdrasil;

import com.mojang.authlib.GameProfile;
import com.mojang.authlib.minecraft.InsecureTextureException;
import com.mojang.authlib.minecraft.MinecraftProfileTexture;
import com.mojang.authlib.minecraft.MinecraftProfileTextures;
import java.util.HashMap;
import java.util.Map;

/**
 * A stand-in for the class the agent patches, carrying both shapes of {@code getTextures} authlib
 * has had: the map from before 1.20.2 and the object from after it.
 */
public class YggdrasilMinecraftSessionService {
    public static final String SERVER_SKIN = "http://textures.example/from-the-server.png";

    public Map<MinecraftProfileTexture.Type, MinecraftProfileTexture> getTextures(
            GameProfile profile, boolean requireSecure) {
        // What authlib does with a signature it cannot check: it throws rather
        // than returning, which is the whole reason the wrapper catches.
        if (requireSecure && profile.hasUnverifiableTextures()) {
            throw new InsecureTextureException("Textures payload has been tampered with (signature invalid)");
        }

        final Map<MinecraftProfileTexture.Type, MinecraftProfileTexture> textures = new HashMap<>();
        if (profile.hasSignedTextures()) {
            textures.put(MinecraftProfileTexture.Type.SKIN, new MinecraftProfileTexture(SERVER_SKIN, null));
        }
        return textures;
    }

    public MinecraftProfileTextures getTextures(GameProfile profile) {
        if (!profile.hasSignedTextures()) {
            return MinecraftProfileTextures.EMPTY;
        }
        return new MinecraftProfileTextures(
                new MinecraftProfileTexture(SERVER_SKIN, null),
                null,
                null,
                MinecraftProfileTextures.SignatureState.SIGNED);
    }
}

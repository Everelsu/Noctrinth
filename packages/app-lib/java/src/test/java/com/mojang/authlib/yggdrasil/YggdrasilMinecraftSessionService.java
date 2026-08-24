package com.mojang.authlib.yggdrasil;

import com.mojang.authlib.GameProfile;
import com.mojang.authlib.minecraft.InsecureTextureException;
import com.mojang.authlib.minecraft.MinecraftProfileTexture;
import com.mojang.authlib.minecraft.MinecraftProfileTextures;
import com.mojang.authlib.properties.Property;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.HashMap;
import java.util.Map;

/**
 * A stand-in for the class the agent patches, carrying every way authlib has answered for a
 * profile's textures: the map from before 1.20.2, the object from after it, and the packed property
 * the client asks for instead from 1.20.2 on.
 */
public class YggdrasilMinecraftSessionService {
    public static final String SERVER_SKIN = "http://textures.example/from-the-server.png";

    /** A skin authlib will not hand over: the wrong signature, or a domain it does not allow. */
    private static final String REFUSED_SKIN = "http://textures.refused/not-allowed.png";

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

    /** The property the profile arrived with, which an offline server never sends. */
    public Property getPackedTextures(GameProfile profile) {
        if (profile.hasSignedTextures()) {
            return new Property("textures", payload(profile.getName(), SERVER_SKIN));
        }
        if (profile.hasUnverifiableTextures()) {
            return new Property("textures", payload(profile.getName(), REFUSED_SKIN));
        }

        return null;
    }

    /**
     * What is inside that property, as far as authlib is willing to say.
     *
     * <p>It answers with nothing at all for a skin it will not accept — a signature from the wrong
     * key, or a URL outside the domains it allows — which is the same nothing an empty payload
     * gets.
     */
    public MinecraftProfileTextures unpackTextures(Property property) {
        final String decoded = new String(Base64.getDecoder().decode(property.value()), StandardCharsets.UTF_8);
        if (!decoded.contains(SERVER_SKIN)) {
            return MinecraftProfileTextures.EMPTY;
        }

        return new MinecraftProfileTextures(
                new MinecraftProfileTexture(SERVER_SKIN, null),
                null,
                null,
                MinecraftProfileTextures.SignatureState.SIGNED);
    }

    private static String payload(String name, String skin) {
        final String json = "{\"profileName\":\"" + name + "\",\"textures\":{\"SKIN\":{\"url\":\"" + skin + "\"}}}";
        return Base64.getEncoder().encodeToString(json.getBytes(StandardCharsets.UTF_8));
    }
}

package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * A skin site that answers by name in CustomSkinLoader's format: {@code /csl/{name}.json}, naming
 * its textures by hash under {@code /textures/{hash}}.
 *
 * <p>LittleSkin is the one that matters: after Ely.by it is where the most players without a
 * Mojang account keep a skin, and the CustomSkinLoader mod is how they see each other. Asked here,
 * everyone sees them without the mod.
 */
final class CustomSkinApiSource implements SkinSource.Source {
    /** What names LittleSkin in {@code noctrinth.skins.source}. */
    static final String LITTLESKIN = "littleskin";

    private static final String LITTLESKIN_URL = "https://littleskin.cn";

    private final String baseUrl;

    CustomSkinApiSource(String baseUrl) {
        this.baseUrl = baseUrl;
    }

    /** The source {@code name} stands for, or null when it is not one of these. */
    static CustomSkinApiSource named(String name) {
        return LITTLESKIN.equalsIgnoreCase(name) ? new CustomSkinApiSource(LITTLESKIN_URL) : null;
    }

    @Override
    public Map<String, SkinSource.Texture> textures(String username) throws Exception {
        return readProfile(Http.getJson(baseUrl + "/csl/" + Http.encode(username) + ".json"), baseUrl);
    }

    /**
     * The textures a CustomSkinLoader profile names.
     *
     * <p>An unknown name is an empty object, and a known one without a skin has a null hash. A
     * {@code slim} skin is the thin-armed model, which the game is told through the texture's
     * metadata.
     */
    static Map<String, SkinSource.Texture> readProfile(JsonElement payload, String baseUrl) {
        if (payload == null || !payload.isJsonObject()) {
            return Collections.emptyMap();
        }

        final JsonObject profile = payload.getAsJsonObject();
        final Map<String, SkinSource.Texture> textures = new LinkedHashMap<>();

        final JsonElement skins = profile.get("skins");
        if (skins != null && skins.isJsonObject()) {
            final String slim = hash(skins.getAsJsonObject().get("slim"));
            final String classic = hash(skins.getAsJsonObject().get("default"));
            if (slim != null) {
                textures.put(
                        "SKIN",
                        new SkinSource.Texture(
                                baseUrl + "/textures/" + slim, Collections.singletonMap("model", "slim")));
            } else if (classic != null) {
                textures.put(
                        "SKIN",
                        new SkinSource.Texture(
                                baseUrl + "/textures/" + classic, Collections.<String, String>emptyMap()));
            }
        }

        final String cape = hash(profile.get("cape"));
        if (cape != null) {
            textures.put(
                    "CAPE",
                    new SkinSource.Texture(baseUrl + "/textures/" + cape, Collections.<String, String>emptyMap()));
        }

        return textures;
    }

    private static String hash(JsonElement value) {
        return value != null && value.isJsonPrimitive() && !value.getAsString().isEmpty() ? value.getAsString() : null;
    }

    @Override
    public String toString() {
        return baseUrl;
    }
}

package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.nio.charset.Charset;
import java.util.Base64;
import java.util.Collections;
import java.util.Map;

/**
 * Mojang's own answer for a name, for the players a skin system does not have.
 *
 * <p>Ely.by proxies Mojang for names it has never heard of, which is what covers licensed players
 * most of the time — but the proxy is not always there, and a licensed player standing in front of
 * an Ely.by player is exactly the case that goes wrong when it is not. Asking Mojang directly is
 * two requests: a name buys an id, and an id buys a profile with the same textures payload in it
 * that a server would have sent.
 */
final class MojangSource implements SkinSource.Source {
    /** What names this one in {@code noctrinth.skins.source}. */
    static final String NAME = "mojang";

    private static final String PROFILES_URL = "https://api.mojang.com/users/profiles/minecraft/";
    private static final String SESSION_URL = "https://sessionserver.mojang.com/session/minecraft/profile/";

    private final String profilesUrl;
    private final String sessionUrl;

    MojangSource() {
        this(PROFILES_URL, SESSION_URL);
    }

    /** Both ends are arguments so that a test can stand somewhere else in for them. */
    MojangSource(String profilesUrl, String sessionUrl) {
        this.profilesUrl = profilesUrl;
        this.sessionUrl = sessionUrl;
    }

    @Override
    public Map<String, SkinSource.Texture> textures(String username) throws Exception {
        final String id = string(Http.getJson(profilesUrl + Http.encode(username)), "id");
        if (id == null) {
            return Collections.emptyMap();
        }

        final JsonElement profile = Http.getJson(sessionUrl + Http.encode(id));
        final String packed = packedTextures(profile);
        if (packed == null) {
            return Collections.emptyMap();
        }

        // The property is the same base64 payload a server sends with a profile:
        // the textures sit one level in, under "textures".
        final String decoded = new String(Base64.getDecoder().decode(packed), Charset.forName("UTF-8"));
        final JsonElement payload = JsonParser.parseString(decoded);
        if (payload == null || !payload.isJsonObject()) {
            return Collections.emptyMap();
        }

        return SkinSource.readTextures(payload.getAsJsonObject().get("textures"));
    }

    /** The value of the {@code textures} property, or null if the profile carries none. */
    private static String packedTextures(JsonElement profile) {
        if (profile == null || !profile.isJsonObject()) {
            return null;
        }

        final JsonElement properties = profile.getAsJsonObject().get("properties");
        if (properties == null || !properties.isJsonArray()) {
            return null;
        }

        for (final JsonElement element : properties.getAsJsonArray()) {
            if (element.isJsonObject() && "textures".equals(string(element, "name"))) {
                return string(element, "value");
            }
        }

        return null;
    }

    private static String string(JsonElement object, String key) {
        if (object == null || !object.isJsonObject()) {
            return null;
        }

        final JsonObject json = object.getAsJsonObject();
        final JsonElement value = json.get(key);
        return value != null && value.isJsonPrimitive() ? value.getAsString() : null;
    }

    @Override
    public String toString() {
        return NAME;
    }
}

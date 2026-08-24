package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import java.util.Map;

/**
 * A skin system that answers by name at {@code /textures/{name}} — the convention Ely.by's
 * {@code skinsystem} serves and the one anybody hosting their own tends to follow.
 */
final class SkinSystemSource implements SkinSource.Source {
    private final String baseUrl;

    SkinSystemSource(String baseUrl) {
        this.baseUrl = baseUrl;
    }

    @Override
    public Map<String, SkinSource.Texture> textures(String username) throws Exception {
        final JsonElement payload = Http.getJson(baseUrl + "/textures/" + Http.encode(username));
        return SkinSource.readTextures(payload);
    }

    @Override
    public String toString() {
        return baseUrl;
    }
}

package com.mojang.authlib.minecraft;

import java.util.Map;

/** The pre-1.20.2 texture, including the nested type the map is keyed by. */
public final class MinecraftProfileTexture {
    public enum Type {
        SKIN,
        CAPE,
        ELYTRA
    }

    private final String url;
    private final Map<String, String> metadata;

    public MinecraftProfileTexture(String url, Map<String, String> metadata) {
        this.url = url;
        this.metadata = metadata;
    }

    public String getUrl() {
        return url;
    }

    public String getMetadata(String key) {
        return metadata == null ? null : metadata.get(key);
    }
}

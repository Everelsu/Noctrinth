package com.mojang.authlib;

/** Stands in for authlib's own class, so the agent's reflection has something to reflect on. */
public final class GameProfile {
    private final String name;
    private final boolean signedTextures;

    public GameProfile(String name, boolean signedTextures) {
        this.name = name;
        this.signedTextures = signedTextures;
    }

    public String getName() {
        return name;
    }

    /** Whether the "server" sent textures along with this profile. */
    public boolean hasSignedTextures() {
        return signedTextures;
    }
}

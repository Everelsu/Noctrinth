package com.mojang.authlib;

/** Stands in for authlib's own class, so the agent's reflection has something to reflect on. */
public final class GameProfile {
    private final String name;
    private final boolean signedTextures;
    private final boolean unverifiableTextures;

    public GameProfile(String name, boolean signedTextures) {
        this(name, signedTextures, false);
    }

    private GameProfile(String name, boolean signedTextures, boolean unverifiableTextures) {
        this.name = name;
        this.signedTextures = signedTextures;
        this.unverifiableTextures = unverifiableTextures;
    }

    /**
     * A profile carrying textures signed by an account system this client has no key for — an
     * Ely.by player as a licensed client receives them, or the other way about.
     */
    public static GameProfile unverifiable(String name) {
        return new GameProfile(name, false, true);
    }

    public String getName() {
        return name;
    }

    /** Whether the "server" sent textures along with this profile. */
    public boolean hasSignedTextures() {
        return signedTextures;
    }

    /** Whether those textures are ones this client would refuse. */
    public boolean hasUnverifiableTextures() {
        return unverifiableTextures;
    }
}

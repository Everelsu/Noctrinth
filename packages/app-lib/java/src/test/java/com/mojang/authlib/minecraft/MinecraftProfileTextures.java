package com.mojang.authlib.minecraft;

/** The 1.20.2+ shape, written as a class so the sources still compile down to Java 8. */
public final class MinecraftProfileTextures {
    public enum SignatureState {
        UNSIGNED,
        INVALID,
        SIGNED
    }

    public static final MinecraftProfileTextures EMPTY =
            new MinecraftProfileTextures(null, null, null, SignatureState.UNSIGNED);

    private final MinecraftProfileTexture skin;
    private final MinecraftProfileTexture cape;
    private final MinecraftProfileTexture elytra;
    private final SignatureState signatureState;

    public MinecraftProfileTextures(
            MinecraftProfileTexture skin,
            MinecraftProfileTexture cape,
            MinecraftProfileTexture elytra,
            SignatureState signatureState) {
        this.skin = skin;
        this.cape = cape;
        this.elytra = elytra;
        this.signatureState = signatureState;
    }

    public MinecraftProfileTexture skin() {
        return skin;
    }

    public MinecraftProfileTexture cape() {
        return cape;
    }

    public MinecraftProfileTexture elytra() {
        return elytra;
    }

    public SignatureState signatureState() {
        return signatureState;
    }
}

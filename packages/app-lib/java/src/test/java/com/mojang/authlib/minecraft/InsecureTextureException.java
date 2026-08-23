package com.mojang.authlib.minecraft;

/** Stands in for what authlib throws for textures it cannot verify the signature of. */
public class InsecureTextureException extends RuntimeException {
    private static final long serialVersionUID = 1L;

    public InsecureTextureException(String message) {
        super(message);
    }
}

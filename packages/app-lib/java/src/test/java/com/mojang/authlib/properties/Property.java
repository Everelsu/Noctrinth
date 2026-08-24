package com.mojang.authlib.properties;

/** Stands in for authlib's own, which is a record from 1.20.2 on — hence {@code value()}. */
public final class Property {
    private final String name;
    private final String value;

    public Property(String name, String value) {
        this.name = name;
        this.value = value;
    }

    public String name() {
        return name;
    }

    public String value() {
        return value;
    }
}

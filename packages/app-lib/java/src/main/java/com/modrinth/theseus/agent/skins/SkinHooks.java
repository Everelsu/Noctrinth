package com.modrinth.theseus.agent.skins;

import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.util.HashMap;
import java.util.Map;

/**
 * The code {@link com.modrinth.theseus.agent.transformers.SessionServiceTransformer} calls from the
 * end of authlib's {@code getTextures}.
 *
 * <p>Everything here is reflective on purpose. The agent is compiled against nothing from the game,
 * and authlib has changed this corner more than once: what used to be a {@code Map} keyed by
 * texture type is a {@code MinecraftProfileTextures} record from 1.20.2 on. Both shapes arrive here
 * as {@code Object}, and whichever one came in is the one that goes back out.
 */
public final class SkinHooks {
    private static final String TEXTURE_CLASS = "com.mojang.authlib.minecraft.MinecraftProfileTexture";
    private static final String TEXTURE_TYPE_CLASS = TEXTURE_CLASS + "$Type";

    /** The signature state to claim for textures we resolved ourselves. */
    private static final String UNSIGNED_CONSTANT = "UNSIGNED";

    private SkinHooks() {}

    /**
     * Fills in textures the server did not send.
     *
     * <p>Called with whatever authlib was about to return. A profile that already carries textures
     * is handed straight back — this only ever speaks for players nobody else answered for, so an
     * online-mode server's signed skins are never second-guessed.
     */
    public static Object fillTextures(Object result, Object profile) {
        try {
            if (!SkinSource.isEnabled() || profile == null || hasTextures(result)) {
                return result;
            }

            final String name = profileName(profile);
            final Map<String, SkinSource.Texture> textures = SkinSource.lookup(name);
            if (textures.isEmpty()) {
                return result;
            }

            final ClassLoader loader = profile.getClass().getClassLoader();
            final Object filled =
                    result instanceof Map ? buildMap(loader, textures) : buildTexturesObject(result, loader, textures);

            if (filled != null) {
                SkinSource.debug("Filled in textures for " + name);
                return filled;
            }
        } catch (Throwable t) {
            // Whatever this is, it is not worth taking the game down with it.
            SkinSource.debug("Failed to fill in textures: " + t);
        }

        return result;
    }

    private static boolean hasTextures(Object result) throws Exception {
        if (result == null) {
            return false;
        }

        if (result instanceof Map) {
            return !((Map<?, ?>) result).isEmpty();
        }

        // The record shape: a component per texture, all null when there are none.
        for (final Method method : result.getClass().getMethods()) {
            if (method.getParameterTypes().length == 0
                    && TEXTURE_CLASS.equals(method.getReturnType().getName())
                    && method.invoke(result) != null) {
                return true;
            }
        }

        return false;
    }

    private static String profileName(Object profile) {
        // `getName` on every authlib a launcher meets; `name` covers the record
        // form, in case that is where GameProfile ends up too.
        for (final String accessor : new String[] {"getName", "name"}) {
            try {
                final Object name = profile.getClass().getMethod(accessor).invoke(profile);
                if (name instanceof String) {
                    return (String) name;
                }
            } catch (Throwable ignored) {
                // Try the next one.
            }
        }

        return null;
    }

    /** The pre-1.20.2 shape: a map keyed by {@code MinecraftProfileTexture.Type}. */
    private static Object buildMap(ClassLoader loader, Map<String, SkinSource.Texture> textures) throws Exception {
        final Class<?> typeClass = Class.forName(TEXTURE_TYPE_CLASS, false, loader);
        final Map<Object, Object> filled = new HashMap<>();

        for (final Map.Entry<String, SkinSource.Texture> entry : textures.entrySet()) {
            final Object type = enumConstant(typeClass, entry.getKey());
            final Object texture = buildTexture(loader, entry.getValue());
            if (type != null && texture != null) {
                filled.put(type, texture);
            }
        }

        return filled.isEmpty() ? null : filled;
    }

    /**
     * The 1.20.2+ shape: a record of skin, cape, elytra and a signature state.
     *
     * <p>Built from the instance authlib was already returning rather than from a class name, so
     * the record it hands back is the one that class expects, and the components are filled in
     * their declared order — which for a record's canonical constructor is the order they are
     * written in.
     */
    private static Object buildTexturesObject(
            Object result, ClassLoader loader, Map<String, SkinSource.Texture> textures) throws Exception {
        if (result == null) {
            return null;
        }

        Constructor<?> canonical = null;
        for (final Constructor<?> candidate : result.getClass().getDeclaredConstructors()) {
            if (canonical == null || candidate.getParameterTypes().length > canonical.getParameterTypes().length) {
                canonical = candidate;
            }
        }
        if (canonical == null) {
            return null;
        }

        final Class<?>[] parameters = canonical.getParameterTypes();
        final Object[] arguments = new Object[parameters.length];
        final String[] order = {"SKIN", "CAPE", "ELYTRA"};
        int textureIndex = 0;

        for (int i = 0; i < parameters.length; i++) {
            final Class<?> parameter = parameters[i];
            if (TEXTURE_CLASS.equals(parameter.getName())) {
                final SkinSource.Texture texture =
                        textureIndex < order.length ? textures.get(order[textureIndex]) : null;
                arguments[i] = texture == null ? null : buildTexture(loader, texture);
                textureIndex++;
            } else if (parameter.isEnum()) {
                arguments[i] = enumConstant(parameter, UNSIGNED_CONSTANT);
                if (arguments[i] == null) {
                    final Object[] constants = parameter.getEnumConstants();
                    arguments[i] = constants.length > 0 ? constants[0] : null;
                }
            } else if (parameter.isPrimitive()) {
                // Nothing sensible to say about it; a default keeps the call legal.
                arguments[i] = defaultValue(parameter);
            }
        }

        if (textureIndex == 0) {
            return null;
        }

        canonical.setAccessible(true);
        return canonical.newInstance(arguments);
    }

    private static Object buildTexture(ClassLoader loader, SkinSource.Texture texture) throws Exception {
        final Class<?> textureClass = Class.forName(TEXTURE_CLASS, false, loader);

        Constructor<?> withMetadata = null;
        Constructor<?> withUrl = null;
        for (final Constructor<?> candidate : textureClass.getDeclaredConstructors()) {
            final Class<?>[] parameters = candidate.getParameterTypes();
            if (parameters.length == 2 && parameters[0] == String.class && parameters[1] == Map.class) {
                withMetadata = candidate;
            } else if (parameters.length == 1 && parameters[0] == String.class) {
                withUrl = candidate;
            }
        }

        if (withMetadata != null) {
            withMetadata.setAccessible(true);
            // Slim arms and the like ride along in here.
            return withMetadata.newInstance(texture.url, texture.metadata.isEmpty() ? null : texture.metadata);
        }
        if (withUrl != null) {
            withUrl.setAccessible(true);
            return withUrl.newInstance(texture.url);
        }

        return null;
    }

    private static Object enumConstant(Class<?> enumClass, String name) {
        final Object[] constants = enumClass.getEnumConstants();
        if (constants == null) {
            return null;
        }

        for (final Object constant : constants) {
            if (constant instanceof Enum && ((Enum<?>) constant).name().equals(name)) {
                return constant;
            }
        }

        return null;
    }

    private static Object defaultValue(Class<?> primitive) {
        if (primitive == boolean.class) {
            return Boolean.FALSE;
        }
        if (primitive == char.class) {
            return (char) 0;
        }
        if (primitive == long.class) {
            return 0L;
        }
        if (primitive == float.class) {
            return 0f;
        }
        if (primitive == double.class) {
            return 0d;
        }
        if (primitive == byte.class) {
            return (byte) 0;
        }
        if (primitive == short.class) {
            return (short) 0;
        }
        return 0;
    }
}

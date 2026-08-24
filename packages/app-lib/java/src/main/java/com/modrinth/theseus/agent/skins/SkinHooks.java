package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.HashMap;
import java.util.Map;

/**
 * The code {@link com.modrinth.theseus.agent.transformers.SessionServiceTransformer} calls from the
 * end of the authlib methods a skin can go missing in.
 *
 * <p>Everything here is reflective on purpose. The agent is compiled against nothing from the game,
 * and authlib has changed this corner more than once: what used to be a {@code Map} keyed by
 * texture type is a {@code MinecraftProfileTextures} record from 1.20.2 on, and the client that
 * used to ask for textures now asks for the property they are packed in. Every shape arrives here
 * as {@code Object}, and whichever one came in is the one that goes back out.
 */
public final class SkinHooks {
    private static final String TEXTURE_CLASS = "com.mojang.authlib.minecraft.MinecraftProfileTexture";
    private static final String TEXTURE_TYPE_CLASS = TEXTURE_CLASS + "$Type";
    private static final String PROPERTY_CLASS = "com.mojang.authlib.properties.Property";

    /** What a packed textures payload calls the player it is for. */
    private static final String PROFILE_NAME = "profileName";

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

            final Object filled = build(result == null ? null : result.getClass(), loaderOf(profile), textures);
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

    /**
     * Stands in for textures authlib refused to hand over.
     *
     * <p>A profile carrying textures signed by an account system this client does not have the key
     * for — an Ely.by player seen by a licensed one, or the other way about — does not come back
     * empty: {@code getTextures} throws, and the game reads that as no skin at all. The player is
     * no less real for it, so they get the same lookup by name as one the server said nothing
     * about.
     *
     * <p>{@code expected} is the type the call site is going to cast this to, which is all there is
     * to go on when the failure left no instance to copy the shape from. Nothing to offer means the
     * original failure is passed on exactly as it was thrown.
     */
    public static Object recoverTextures(Throwable failure, Object profile, Class<?> expected) {
        try {
            if (SkinSource.isEnabled() && profile != null) {
                final String name = profileName(profile);
                final Map<String, SkinSource.Texture> textures = SkinSource.lookup(name);

                final Object filled = textures.isEmpty() ? null : build(expected, loaderOf(profile), textures);
                if (filled != null) {
                    SkinSource.debug("Filled in textures for " + name + " after " + failure);
                    return filled;
                }
            }
        } catch (Throwable t) {
            SkinSource.debug("Failed to fill in textures after " + failure + ": " + t);
        }

        throw sneakyThrow(failure);
    }

    /**
     * Gives the client a textures property to unpack for a profile that arrived without one.
     *
     * <p>From 1.20.2 the client asks for the property first and only unpacks it if there was one,
     * so a profile from an offline server — no property at all — never reaches {@link
     * #unpackTextures}, which is where a lookup belongs. This puts an empty one in its way instead,
     * naming the player it is for, and no lookup happens here: this is called while the client is
     * deciding what to draw, and the unpacking is what runs off the main thread.
     */
    public static Object packTextures(Object result, Object profile) {
        try {
            if (result != null || !SkinSource.isEnabled() || profile == null) {
                return result;
            }

            final String name = profileName(profile);
            if (name == null) {
                return result;
            }

            final Object property = buildProperty(loaderOf(profile), name);
            if (property != null) {
                return property;
            }
        } catch (Throwable t) {
            SkinSource.debug("Failed to stand in for a missing textures property: " + t);
        }

        return result;
    }

    /**
     * Fills in textures the property did not yield.
     *
     * <p>Which covers more than the empty property {@link #packTextures} makes: authlib drops
     * textures whose signature it cannot check and textures hosted anywhere but Mojang's own
     * domains, so an Ely.by player's skin arrives here as nothing at all on a licensed client. The
     * name is read back out of the property, which carries it whether the payload came from a
     * server or from us.
     */
    public static Object unpackTextures(Object result, Object property) {
        try {
            if (!SkinSource.isEnabled() || result == null || hasTextures(result)) {
                return result;
            }

            final String name = payloadName(propertyValue(property));
            if (name == null) {
                return result;
            }

            final Map<String, SkinSource.Texture> textures = SkinSource.lookup(name);
            final Object filled = textures.isEmpty() ? null : build(result.getClass(), loaderOf(result), textures);
            if (filled != null) {
                SkinSource.debug("Filled in textures for " + name);
                return filled;
            }
        } catch (Throwable t) {
            SkinSource.debug("Failed to fill in unpacked textures: " + t);
        }

        return result;
    }

    /** An empty textures property naming the player it stands for. */
    private static Object buildProperty(ClassLoader loader, String name) throws Exception {
        final Class<?> propertyClass = Class.forName(PROPERTY_CLASS, false, loader);

        Constructor<?> canonical = null;
        for (final Constructor<?> candidate : propertyClass.getDeclaredConstructors()) {
            final Class<?>[] parameters = candidate.getParameterTypes();
            if (parameters.length < 2 || parameters.length > 3) {
                continue;
            }

            boolean strings = true;
            for (final Class<?> parameter : parameters) {
                strings &= parameter == String.class;
            }

            // The shortest one that takes only strings: name and value, without a
            // signature there is nothing to put in.
            if (strings && (canonical == null || parameters.length < canonical.getParameterTypes().length)) {
                canonical = candidate;
            }
        }

        if (canonical == null) {
            return null;
        }

        final Object[] arguments = new Object[canonical.getParameterTypes().length];
        arguments[0] = "textures";
        arguments[1] = emptyPayload(name);

        canonical.setAccessible(true);
        return canonical.newInstance(arguments);
    }

    /** The payload a server would have sent, with the textures left out. */
    private static String emptyPayload(String name) {
        final JsonObject payload = new JsonObject();
        payload.addProperty(PROFILE_NAME, name);
        payload.add("textures", new JsonObject());

        return Base64.getEncoder().encodeToString(payload.toString().getBytes(StandardCharsets.UTF_8));
    }

    /** The {@code value} of a property, whichever accessor this authlib gave it. */
    private static String propertyValue(Object property) {
        if (property == null) {
            return null;
        }

        for (final String accessor : new String[] {"value", "getValue"}) {
            try {
                final Object value = property.getClass().getMethod(accessor).invoke(property);
                if (value instanceof String) {
                    return (String) value;
                }
            } catch (Throwable ignored) {
                // Try the next one.
            }
        }

        return null;
    }

    /** The name a packed textures payload was made for. */
    private static String payloadName(String packed) {
        if (packed == null) {
            return null;
        }

        try {
            final String decoded = new String(Base64.getDecoder().decode(packed), StandardCharsets.UTF_8);
            final JsonElement payload = JsonParser.parseString(decoded);
            if (payload == null || !payload.isJsonObject()) {
                return null;
            }

            final JsonElement name = payload.getAsJsonObject().get(PROFILE_NAME);
            return name != null && name.isJsonPrimitive() ? name.getAsString() : null;
        } catch (Throwable t) {
            // Not a payload, then; there is no name to be had.
            return null;
        }
    }

    private static ClassLoader loaderOf(Object object) {
        return object.getClass().getClassLoader();
    }

    /** Builds whichever shape of the API {@code shape} belongs to, or null for neither. */
    private static Object build(Class<?> shape, ClassLoader loader, Map<String, SkinSource.Texture> textures)
            throws Exception {
        if (shape == null) {
            return null;
        }

        return shape.isAssignableFrom(HashMap.class)
                ? buildMap(loader, textures)
                : buildTexturesObject(shape, loader, textures);
    }

    /**
     * Throws what it is given, whatever it is.
     *
     * <p>The declared throws is erased, so the compiler lets a checked exception through without
     * this method having to admit to it — which is what passing a failure on untouched needs.
     */
    @SuppressWarnings("unchecked")
    private static <T extends Throwable> RuntimeException sneakyThrow(Throwable failure) throws T {
        throw (T) failure;
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
     * <p>Built from the class authlib itself deals in — the one it was returning, or the one the
     * call site is about to cast to — so the record it hands back is the one that class expects,
     * and the components are filled in their declared order, which for a record's canonical
     * constructor is the order they are written in.
     */
    private static Object buildTexturesObject(
            Class<?> texturesClass, ClassLoader loader, Map<String, SkinSource.Texture> textures) throws Exception {
        Constructor<?> canonical = null;
        for (final Constructor<?> candidate : texturesClass.getDeclaredConstructors()) {
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

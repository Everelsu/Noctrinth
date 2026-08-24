package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Looks a player's textures up by name, for players the server did not supply any for.
 *
 * <p>Servers running in offline mode hand out profiles with no {@code textures} property at all, so
 * every player renders as Steve no matter whose account they are on. The launcher points this at a
 * skin system that answers by name — Ely.by by default, which serves its own users' skins — with
 * Mojang itself behind it for the licensed players Ely.by has never heard of.
 *
 * <p>Disabled unless the launcher sets {@code noctrinth.skins.source}. Its value is a list
 * separated by commas: the base URL of anything serving {@code /textures/{name}}, or the word
 * {@code mojang} for Mojang's own name lookup. They are asked in the order given and the first one
 * that has heard of the player wins, which is how a private skin server can sit in front of the
 * public one.
 */
public final class SkinSource {
    private static final String SOURCE_PROPERTY = "noctrinth.skins.source";
    private static final String DEBUG_PROPERTY = "modrinth.debugAgent";

    /** Minecraft's own limit on how long a name can be. */
    private static final int MAX_NAME_LENGTH = 16;

    /**
     * How long an answer is reused.
     *
     * <p>Short, because changing a skin and rejoining to see it is how everyone does it, and an
     * answer older than that is the only thing standing in the way. It costs little: the game asks
     * when a player comes into view, not while they are in it, so this bounds how stale a skin can
     * be far more than it bounds how often anyone is asked.
     *
     * <p>A miss is remembered for longer. Whoever this is has no skin anywhere right now, and the
     * lookup that found nothing is the expensive one to repeat.
     */
    private static final long HIT_TTL_MS = 15 * 1000L;

    private static final long MISS_TTL_MS = 60 * 1000L;

    /** A bound on the cache, so a busy server cannot grow it without end. */
    private static final int MAX_CACHED_NAMES = 512;

    private static final List<Source> SOURCES = parseSources(System.getProperty(SOURCE_PROPERTY));
    private static final boolean DEBUG = Boolean.getBoolean(DEBUG_PROPERTY);

    private static final Map<String, CachedTextures> CACHE = new ConcurrentHashMap<>();

    private SkinSource() {}

    public static boolean isEnabled() {
        return !SOURCES.isEmpty();
    }

    /** Somewhere textures can be asked for by name. */
    interface Source {
        /** What this source has for the name, or an empty map if it has never heard of them. */
        Map<String, Texture> textures(String username) throws Exception;
    }

    /**
     * The textures the skin system has for this name, keyed by {@code SKIN} / {@code CAPE}.
     *
     * <p>Never throws and never returns null: a name we cannot resolve, for any reason, is an empty
     * map and the caller leaves the profile as it was.
     */
    public static Map<String, Texture> lookup(String username) {
        if (SOURCES.isEmpty() || !isPlausibleName(username)) {
            return Collections.emptyMap();
        }

        final long now = System.currentTimeMillis();
        final CachedTextures cached = CACHE.get(username);
        if (cached != null && cached.expiresAt > now) {
            return cached.textures;
        }

        Map<String, Texture> textures = Collections.emptyMap();
        for (final Source source : SOURCES) {
            try {
                textures = source.textures(username);
            } catch (Throwable t) {
                // A skin is not worth interrupting the game over, whatever went
                // wrong; the next source may still know them.
                debug("Failed to look up textures for " + username + " at " + source + ": " + t);
                textures = Collections.emptyMap();
            }

            if (!textures.isEmpty()) {
                break;
            }
        }

        store(username, textures, now);
        return textures;
    }

    /**
     * Reads the {@code {"SKIN": {"url": ..., "metadata": {...}}}} shape Mojang's own profile
     * endpoint uses, which the skin systems answering by name mirror.
     */
    static Map<String, Texture> readTextures(JsonElement payload) {
        if (payload == null || !payload.isJsonObject()) {
            return Collections.emptyMap();
        }

        final Map<String, Texture> textures = new LinkedHashMap<>();
        for (final Map.Entry<String, JsonElement> entry :
                payload.getAsJsonObject().entrySet()) {
            if (!entry.getValue().isJsonObject()) {
                continue;
            }

            final JsonObject texture = entry.getValue().getAsJsonObject();
            final JsonElement url = texture.get("url");
            if (url == null || !url.isJsonPrimitive()) {
                continue;
            }

            final Map<String, String> metadata = new LinkedHashMap<>();
            final JsonElement rawMetadata = texture.get("metadata");
            if (rawMetadata != null && rawMetadata.isJsonObject()) {
                for (final Map.Entry<String, JsonElement> meta :
                        rawMetadata.getAsJsonObject().entrySet()) {
                    if (meta.getValue().isJsonPrimitive()) {
                        metadata.put(meta.getKey(), meta.getValue().getAsString());
                    }
                }
            }

            textures.put(entry.getKey().toUpperCase(Locale.ROOT), new Texture(url.getAsString(), metadata));
        }

        return textures;
    }

    private static void store(String username, Map<String, Texture> textures, long now) {
        if (CACHE.size() >= MAX_CACHED_NAMES) {
            CACHE.values().removeIf(entry -> entry.expiresAt <= now);
            if (CACHE.size() >= MAX_CACHED_NAMES) {
                CACHE.clear();
            }
        }

        CACHE.put(username, new CachedTextures(textures, now + (textures.isEmpty() ? MISS_TTL_MS : HIT_TTL_MS)));
    }

    /**
     * Whether this is worth spending a request on.
     *
     * <p>Anything a server can hand out fits. Mojang's own names are letters, digits and
     * underscores, but an offline server hands out whatever it was given and account systems of
     * their own are looser — Ely.by allows a dash, among others — so the only names turned away
     * here are the ones that could not be asked about safely: empty, over-long, or carrying a path
     * separator or a control character.
     */
    private static boolean isPlausibleName(String username) {
        if (username == null || username.isEmpty() || username.length() > MAX_NAME_LENGTH) {
            return false;
        }

        for (int i = 0; i < username.length(); i++) {
            final char c = username.charAt(i);
            if (c < ' ' || c == 127 || c == '/' || c == '\\' || Character.isWhitespace(c)) {
                return false;
            }
        }

        return true;
    }

    /** Reads the sources out of what the launcher passed, in the order it listed them. */
    static List<Source> parseSources(String configured) {
        if (configured == null || configured.trim().isEmpty()) {
            return Collections.emptyList();
        }

        final List<Source> sources = new ArrayList<>();
        for (String entry : configured.split(",")) {
            entry = entry.trim();
            while (entry.endsWith("/")) {
                entry = entry.substring(0, entry.length() - 1);
            }

            if (entry.isEmpty()) {
                continue;
            }

            sources.add(MojangSource.NAME.equalsIgnoreCase(entry) ? new MojangSource() : new SkinSystemSource(entry));
        }

        return Collections.unmodifiableList(sources);
    }

    static void debug(String message) {
        if (DEBUG) {
            System.out.println("[noctrinth-skins] " + message);
        }
    }

    /** One texture as the skin system describes it. */
    public static final class Texture {
        public final String url;
        public final Map<String, String> metadata;

        Texture(String url, Map<String, String> metadata) {
            this.url = url;
            this.metadata = metadata;
        }
    }

    private static final class CachedTextures {
        final Map<String, Texture> textures;
        final long expiresAt;

        CachedTextures(Map<String, Texture> textures, long expiresAt) {
            this.textures = textures;
            this.expiresAt = expiresAt;
        }
    }
}

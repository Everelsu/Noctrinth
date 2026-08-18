package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.Charset;
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
 * skin system that answers by name — Ely.by by default, which serves its own users' skins and
 * proxies Mojang's for everyone else, so one lookup covers licensed and offline players alike.
 *
 * <p>Disabled unless the launcher sets {@code noctrinth.skins.source}. Its value is a base URL, or
 * several separated by commas: they are asked in the order given and the first one that has heard
 * of the player wins, which is how a private skin server can sit in front of the public one.
 */
public final class SkinSource {
    private static final String SOURCE_PROPERTY = "noctrinth.skins.source";
    private static final String DEBUG_PROPERTY = "modrinth.debugAgent";

    /** Minecraft's own limits: at most 16 characters, and only these. */
    private static final int MAX_NAME_LENGTH = 16;

    private static final int CONNECT_TIMEOUT_MS = 4000;
    private static final int READ_TIMEOUT_MS = 4000;

    /**
     * How long an answer is reused.
     *
     * <p>A miss is remembered too, and for much less time: whoever the server sent us has no skin
     * anywhere right now, and asking again for every frame they are on screen would be worse than
     * being wrong for two minutes.
     */
    private static final long HIT_TTL_MS = 10 * 60 * 1000L;

    private static final long MISS_TTL_MS = 2 * 60 * 1000L;

    /** A bound on the cache, so a busy server cannot grow it without end. */
    private static final int MAX_CACHED_NAMES = 512;

    private static final List<String> BASE_URLS = readBaseUrls();
    private static final boolean DEBUG = Boolean.getBoolean(DEBUG_PROPERTY);

    private static final Map<String, CachedTextures> CACHE = new ConcurrentHashMap<>();

    private SkinSource() {}

    public static boolean isEnabled() {
        return !BASE_URLS.isEmpty();
    }

    /**
     * The textures the skin system has for this name, keyed by {@code SKIN} / {@code CAPE}.
     *
     * <p>Never throws and never returns null: a name we cannot resolve, for any reason, is an empty
     * map and the caller leaves the profile as it was.
     */
    public static Map<String, Texture> lookup(String username) {
        if (BASE_URLS.isEmpty() || !isPlausibleName(username)) {
            return Collections.emptyMap();
        }

        final long now = System.currentTimeMillis();
        final CachedTextures cached = CACHE.get(username);
        if (cached != null && cached.expiresAt > now) {
            return cached.textures;
        }

        Map<String, Texture> textures = Collections.emptyMap();
        for (final String baseUrl : BASE_URLS) {
            try {
                textures = fetch(baseUrl, username);
            } catch (Throwable t) {
                // A skin is not worth interrupting the game over, whatever went
                // wrong; the next source may still know them.
                debug("Failed to look up textures for " + username + " at " + baseUrl + ": " + t);
                textures = Collections.emptyMap();
            }

            if (!textures.isEmpty()) {
                break;
            }
        }

        store(username, textures, now);
        return textures;
    }

    private static Map<String, Texture> fetch(String baseUrl, String username) throws Exception {
        final URL url = new URL(baseUrl + "/textures/" + URLEncoder.encode(username, "UTF-8"));
        final HttpURLConnection connection = (HttpURLConnection) url.openConnection();
        connection.setConnectTimeout(CONNECT_TIMEOUT_MS);
        connection.setReadTimeout(READ_TIMEOUT_MS);
        connection.setRequestProperty("Accept", "application/json");
        connection.setRequestProperty("User-Agent", "Noctrinth");

        try {
            final int status = connection.getResponseCode();
            // 204 is how Ely.by says "no such player", and any other non-200 is
            // not something we can read either.
            if (status != HttpURLConnection.HTTP_OK) {
                debug("Skin system answered " + status + " for " + username);
                return Collections.emptyMap();
            }

            try (InputStream stream = connection.getInputStream();
                    Reader reader = new InputStreamReader(stream, Charset.forName("UTF-8"))) {
                return parse(JsonParser.parseReader(reader));
            }
        } finally {
            connection.disconnect();
        }
    }

    /**
     * Reads the {@code {"SKIN": {"url": ..., "metadata": {...}}}} shape Mojang's own profile
     * endpoint uses, which the skin systems answering by name mirror.
     */
    private static Map<String, Texture> parse(JsonElement payload) {
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
     * <p>Anything a vanilla server can hand out fits; the check exists so that names carrying
     * slashes or other surprises never reach the skin system.
     */
    private static boolean isPlausibleName(String username) {
        if (username == null || username.isEmpty() || username.length() > MAX_NAME_LENGTH) {
            return false;
        }

        for (int i = 0; i < username.length(); i++) {
            final char c = username.charAt(i);
            final boolean allowed =
                    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_';
            if (!allowed) {
                return false;
            }
        }

        return true;
    }

    private static List<String> readBaseUrls() {
        final String configured = System.getProperty(SOURCE_PROPERTY);
        if (configured == null || configured.trim().isEmpty()) {
            return Collections.emptyList();
        }

        final List<String> urls = new ArrayList<>();
        for (String base : configured.split(",")) {
            base = base.trim();
            while (base.endsWith("/")) {
                base = base.substring(0, base.length() - 1);
            }
            if (!base.isEmpty()) {
                urls.add(base);
            }
        }

        return Collections.unmodifiableList(urls);
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

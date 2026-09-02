package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.FutureTask;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

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
 * {@code mojang} for Mojang's own name lookup. The first one that has heard of the player wins,
 * which is how a private skin server can sit in front of the public one — but they are all asked
 * at once rather than in turn, because the game does not wait for ever and the player who needs
 * the source at the end of the list is the one who would never be reached in time. See {@link
 * #PER_SOURCE_WAIT_MS}.
 */
public final class SkinSource {
    private static final String SOURCE_PROPERTY = "noctrinth.skins.source";
    private static final String LOCAL_PROPERTY = "noctrinth.skins.local";
    private static final String CACHE_PROPERTY = "noctrinth.skins.cache";
    private static final String DEBUG_PROPERTY = "modrinth.debugAgent";

    /** Minecraft's own limit on how long a name can be. */
    private static final int MAX_NAME_LENGTH = 16;

    /**
     * How long an answer is current for.
     *
     * <p>Short, because changing a skin and rejoining to see it is how everyone does it, and an
     * answer older than that is the only thing standing in the way. Going stale is not the same as
     * being useless, though — see {@link #KEEP_MS}.
     */
    private static final long FRESH_MS = 15 * 1000L;

    /**
     * The same, for a name nothing had a skin for.
     *
     * <p>Longer: whoever this is has no skin anywhere right now, and the lookup that found nothing
     * is the expensive one to repeat.
     */
    private static final long FRESH_MISS_MS = 60 * 1000L;

    /**
     * How long a stale answer is still worth handing over.
     *
     * <p>The game asks when a player comes into view, and it asks on the thread that is waiting to
     * draw them — two threads for the whole game on the older versions. Making it wait on the
     * network is what puts Steve on screen for a second every time somebody walks back into
     * render distance, or every time a death drops the world and every player in it. So anything
     * remembered is handed over at once and looked at again behind the game's back: the skin on
     * screen is at worst one sighting out of date, and there is no pause before it appears.
     */
    private static final long KEEP_MS = 30 * 60 * 1000L;

    /**
     * The longest the game is ever made to wait on a name nothing is known about.
     *
     * <p>Each source has its own timeouts, and a bad minute on the network can put them end to end:
     * a source that will not connect, then one that will not answer, is ten seconds of a frozen
     * frame on the older versions. Past this the lookup is left running on its own and the player
     * is Steve for one sighting — by the next one the answer is in.
     */
    private static final long FIRST_WAIT_MS = 5 * 1000L;

    /**
     * The longest any one source is waited on before the answer of the one after it will do.
     *
     * <p>Preferring an earlier source cannot mean waiting on it without end: a skin system that
     * proxies Mojang for the players it does not have of its own answers for a licensed player
     * most of the time and hangs or fails the rest of it, and every second of that is spent
     * against {@link #FIRST_WAIT_MS}. Past this, whoever else has answered is the answer, and the
     * slow one is still remembered for the sighting after this.
     */
    private static final long PER_SOURCE_WAIT_MS = 3 * 1000L;

    /**
     * The longest a source is given once nobody else has answered either.
     *
     * <p>Reached only when the quick pass over them all came back with nothing, and by then
     * whoever asked has their answer — an empty one — and is not waiting on this. What it is for is
     * the remembering: a source that was merely slow still gets to say what it knows, and the next
     * time this player is looked at the answer is already in. Long enough for any source to have
     * hit a timeout of its own; it is not a substitute for one.
     */
    private static final long LATE_WAIT_MS = 20 * 1000L;

    /**
     * How long a source that failed is left out of the next lookups.
     *
     * <p>Only ever when somebody else can answer. A source that is down stays down for longer than
     * one player walks into view, and asking it again for every one of them costs a connection and
     * a wait each time — while the source that could have answered is the one being kept waiting.
     */
    private static final long UNHEALTHY_MS = 60 * 1000L;

    /** A bound on the cache, so a busy server cannot grow it without end. */
    private static final int MAX_CACHED_NAMES = 512;

    /**
     * How old an answer written to disk may be and still be worth reading back.
     *
     * <p>Generous, because reading one is what makes the first sighting of a session instant, and
     * it is only ever handed over as something to look at again — never as the last word.
     */
    private static final long DISK_TTL_MS = 24 * 60 * 60 * 1000L;

    /** How often the file on disk is rewritten while the game is running. */
    private static final long SAVE_EVERY_MS = 60 * 1000L;

    private static final List<Source> SOURCES = parseSources(System.getProperty(SOURCE_PROPERTY));
    private static final boolean DEBUG = Boolean.getBoolean(DEBUG_PROPERTY);

    private static final Map<String, CachedTextures> CACHE = new ConcurrentHashMap<>();

    /** Sources that have just failed, and until when they are left alone. */
    private static final Map<Source, Long> UNHEALTHY_UNTIL = new ConcurrentHashMap<>();

    /** Names a refresh is already running for, so a crowd cannot ask twice over. */
    private static final Set<String> IN_FLIGHT = ConcurrentHashMap.newKeySet();

    /**
     * Where the looking-again happens.
     *
     * <p>Daemon threads, because none of this is worth holding the game open at the end; a bounded
     * queue and a silent discard, because a refresh that cannot be run right now is one the next
     * sighting will ask for again anyway.
     */
    private static final ExecutorService REFRESHERS = new ThreadPoolExecutor(
            0,
            8,
            30L,
            TimeUnit.SECONDS,
            new LinkedBlockingQueue<>(64),
            runnable -> {
                final Thread thread = new Thread(runnable, "noctrinth-skins");
                thread.setDaemon(true);
                return thread;
            },
            // Refused rather than dropped, so whoever asked knows to do it itself.
            new ThreadPoolExecutor.AbortPolicy());

    /**
     * Where the sources are asked, kept apart from the looking-again above.
     *
     * <p>A lookup waits on the sources it started, so running both on the same threads would have
     * a queue full of lookups waiting for the sources behind them in it. Everything here is a
     * socket waiting for an answer, which is why there can be a good few of them.
     */
    private static final ExecutorService LOOKUPS = new ThreadPoolExecutor(
            0,
            16,
            30L,
            TimeUnit.SECONDS,
            new LinkedBlockingQueue<>(64),
            runnable -> {
                final Thread thread = new Thread(runnable, "noctrinth-skins-source");
                thread.setDaemon(true);
                return thread;
            },
            new ThreadPoolExecutor.AbortPolicy());

    /** When the file on disk was last written, and whether it is behind what is in memory. */
    private static volatile long lastSaved;

    private static volatile boolean dirty;

    static {
        restore();
        if (cacheFile() != null) {
            Runtime.getRuntime().addShutdownHook(new Thread(SkinSource::save, "noctrinth-skins-save"));
        }
    }

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
        if (cached != null && cached.usableUntil > now) {
            if (cached.freshUntil <= now) {
                // Old enough to be worth another look, not old enough to make
                // anyone wait for it.
                refreshLater(username);
            }
            return cached.textures;
        }

        return fetchWithin(username);
    }

    /**
     * Asks about a name nobody has asked about before, without letting the game wait for ever.
     *
     * <p>The work runs on this class's own threads rather than the caller's, so that giving up on
     * it is possible at all: what is abandoned here still finishes, and is still written down.
     */
    private static Map<String, Texture> fetchWithin(String username) {
        final FutureTask<Map<String, Texture>> task = new FutureTask<>(() -> fetch(username));
        try {
            REFRESHERS.execute(task);
        } catch (Throwable rejected) {
            // Every thread is busy; there is nowhere to run this but here.
            task.run();
        }

        try {
            return task.get(FIRST_WAIT_MS, TimeUnit.MILLISECONDS);
        } catch (TimeoutException slow) {
            debug("Gave up waiting on " + username + "; the lookup is still going");
        } catch (Throwable t) {
            debug("Failed to look up " + username + ": " + t);
        }

        return Collections.emptyMap();
    }

    /**
     * Looks a name up before anything asks about it.
     *
     * <p>The launcher names the player who is signing in, and their own skin is the one the game
     * wants first and most visibly — their arm is on screen the moment they spawn. Warming it here
     * means that first sighting is not the one that has to wait.
     */
    public static void prefetch(String username) {
        if (SOURCES.isEmpty() || !isPlausibleName(username)) {
            return;
        }

        refreshLater(username);
    }

    /** Asks every source at once, and remembers what the first one to know answered. */
    private static Map<String, Texture> fetch(String username) {
        final List<Source> asked = healthy();
        final List<FutureTask<Map<String, Texture>>> answers = new ArrayList<>(asked.size());

        for (final Source source : asked) {
            final FutureTask<Map<String, Texture>> answer = new FutureTask<>(() -> ask(source, username));
            answers.add(answer);

            try {
                LOOKUPS.execute(answer);
            } catch (Throwable rejected) {
                // Every thread is busy; there is nowhere to run this but here.
                answer.run();
            }
        }

        // Quickly first, so that one source taking its time cannot keep the
        // answer another one already has from the player waiting to be drawn.
        Map<String, Texture> textures = collect(username, asked, answers, PER_SOURCE_WAIT_MS);

        // Then patiently, for what was only slow. Whoever asked has long since
        // been answered; this is what gets remembered.
        if (textures.isEmpty()) {
            textures = collect(username, asked, answers, LATE_WAIT_MS);
        }

        store(username, textures, System.currentTimeMillis());
        return textures;
    }

    /**
     * What the first source to know has, waiting no longer than {@code wait} on any one of them.
     *
     * <p>Asked in the order they were listed however they finish, so that a skin server put in
     * front of another is still the one that speaks for a player they both know.
     */
    private static Map<String, Texture> collect(
            String username, List<Source> asked, List<FutureTask<Map<String, Texture>>> answers, long wait) {
        for (int i = 0; i < answers.size(); i++) {
            Map<String, Texture> answer = Collections.emptyMap();
            try {
                answer = answers.get(i).get(wait, TimeUnit.MILLISECONDS);
            } catch (TimeoutException slow) {
                // Left running. Being slow is not being wrong, so this is not
                // held against the source: only a failure is.
                debug("Gave up waiting on " + asked.get(i) + " for " + username);
            } catch (Throwable t) {
                debug("Failed to look up " + username + " at " + asked.get(i) + ": " + t);
            }

            if (!answer.isEmpty()) {
                return answer;
            }
        }

        return Collections.emptyMap();
    }

    /** What one source has for a name, or nothing at all if it could not say. */
    private static Map<String, Texture> ask(Source source, String username) {
        try {
            return source.textures(username);
        } catch (Throwable t) {
            // A skin is not worth interrupting the game over, whatever went
            // wrong; another source may still know them.
            debug("Failed to look up textures for " + username + " at " + source + ": " + t);
            markUnhealthy(source);
            return Collections.emptyMap();
        }
    }

    /** The sources worth asking right now, which is all of them unless one is failing. */
    private static List<Source> healthy() {
        final long now = System.currentTimeMillis();
        final List<Source> healthy = new ArrayList<>(SOURCES.size());

        for (final Source source : SOURCES) {
            final Long until = UNHEALTHY_UNTIL.get(source);
            if (until == null || until <= now) {
                healthy.add(source);
            }
        }

        // Nobody is well: ask anyway, since the alternative is answering for
        // nobody at all until one of them recovers unasked.
        return healthy.isEmpty() ? SOURCES : healthy;
    }

    private static void markUnhealthy(Source source) {
        UNHEALTHY_UNTIL.put(source, System.currentTimeMillis() + UNHEALTHY_MS);
    }

    private static void refreshLater(String username) {
        if (!IN_FLIGHT.add(username)) {
            // Somebody is already on it.
            return;
        }

        try {
            REFRESHERS.execute(() -> {
                try {
                    fetch(username);
                    // On this thread rather than the game's, and rarely: whoever
                    // else is asking is waiting to draw somebody.
                    if (dirty && System.currentTimeMillis() - lastSaved > SAVE_EVERY_MS) {
                        save();
                    }
                } finally {
                    IN_FLIGHT.remove(username);
                }
            });
        } catch (Throwable t) {
            IN_FLIGHT.remove(username);
            debug("Could not look " + username + " up in the background: " + t);
        }
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
            CACHE.values().removeIf(entry -> entry.usableUntil <= now);
            if (CACHE.size() >= MAX_CACHED_NAMES) {
                CACHE.clear();
            }
        }

        CACHE.put(
                username,
                new CachedTextures(
                        textures, now, now + (textures.isEmpty() ? FRESH_MISS_MS : FRESH_MS), now + KEEP_MS));

        if (!textures.isEmpty()) {
            dirty = true;
        }
    }

    private static Path cacheFile() {
        final String configured = System.getProperty(CACHE_PROPERTY);
        return configured == null || configured.trim().isEmpty() ? null : Paths.get(configured.trim());
    }

    /**
     * Reads back what the last run knew.
     *
     * <p>Everything read is stale on arrival, by design: it goes on screen the moment somebody is
     * looked at and is checked again behind the game's back. That is the difference between a
     * session that starts with everyone as Steve for a second and one that does not.
     */
    private static void restore() {
        final Path file = cacheFile();
        if (file == null) {
            return;
        }

        try {
            if (!Files.isRegularFile(file)) {
                return;
            }

            final JsonElement root =
                    JsonParser.parseString(new String(Files.readAllBytes(file), StandardCharsets.UTF_8));
            if (root == null || !root.isJsonObject()) {
                return;
            }

            final long now = System.currentTimeMillis();
            for (final Map.Entry<String, JsonElement> entry :
                    root.getAsJsonObject().entrySet()) {
                if (!entry.getValue().isJsonObject()) {
                    continue;
                }

                final JsonObject remembered = entry.getValue().getAsJsonObject();
                final JsonElement savedAt = remembered.get("savedAt");
                if (savedAt == null || !savedAt.isJsonPrimitive() || now - savedAt.getAsLong() > DISK_TTL_MS) {
                    continue;
                }

                final Map<String, Texture> textures = readTextures(remembered.get("textures"));
                if (!textures.isEmpty()) {
                    CACHE.put(entry.getKey(), new CachedTextures(textures, savedAt.getAsLong(), 0L, now + KEEP_MS));
                }
            }

            debug("Remembered " + CACHE.size() + " skins from the last run");
        } catch (Throwable t) {
            debug("Could not read back what was remembered: " + t);
        }
    }

    /** Writes what is worth remembering, whole, over what was there. */
    private static synchronized void save() {
        final Path file = cacheFile();
        if (file == null) {
            return;
        }

        try {
            final JsonObject root = new JsonObject();
            for (final Map.Entry<String, CachedTextures> entry : CACHE.entrySet()) {
                // A name nothing had a skin for is not worth carrying over: it
                // costs a lookup either way, and it may not be true tomorrow.
                if (entry.getValue().textures.isEmpty()) {
                    continue;
                }

                final JsonObject remembered = new JsonObject();
                remembered.addProperty("savedAt", entry.getValue().savedAt);
                remembered.add("textures", asJson(entry.getValue().textures));
                root.add(entry.getKey(), remembered);
            }

            final Path parent = file.getParent();
            if (parent != null) {
                Files.createDirectories(parent);
            }

            // Through a temporary file, so a game that dies mid-write leaves the
            // last good one behind rather than half of this one.
            final Path partial = file.resolveSibling(file.getFileName() + ".part");
            Files.write(partial, root.toString().getBytes(StandardCharsets.UTF_8));
            Files.move(partial, file, StandardCopyOption.REPLACE_EXISTING);

            lastSaved = System.currentTimeMillis();
            dirty = false;
        } catch (Throwable t) {
            debug("Could not write down what was looked up: " + t);
        }
    }

    /** The same shape that is read back, and the one the skin systems answer in. */
    private static JsonObject asJson(Map<String, Texture> textures) {
        final JsonObject json = new JsonObject();
        for (final Map.Entry<String, Texture> entry : textures.entrySet()) {
            final JsonObject texture = new JsonObject();
            texture.addProperty("url", entry.getValue().url);

            if (!entry.getValue().metadata.isEmpty()) {
                final JsonObject metadata = new JsonObject();
                for (final Map.Entry<String, String> value :
                        entry.getValue().metadata.entrySet()) {
                    metadata.addProperty(value.getKey(), value.getValue());
                }
                texture.add("metadata", metadata);
            }

            json.add(entry.getKey(), texture);
        }

        return json;
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

    /**
     * Reads the sources out of what the launcher passed, in the order it listed them.
     *
     * <p>The folder of skins put there by hand comes before all of them, so that dropping a file
     * in it settles the matter for that player whatever any skin system has to say.
     */
    static List<Source> parseSources(String configured) {
        final List<Source> sources = new ArrayList<>();

        final String local = System.getProperty(LOCAL_PROPERTY);
        if (local != null && !local.trim().isEmpty()) {
            sources.add(new LocalSource(Paths.get(local.trim())));
        }

        if (configured == null || configured.trim().isEmpty()) {
            return Collections.unmodifiableList(sources);
        }

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

        /** When this was looked up, which is what goes on disk and ages it there. */
        final long savedAt;

        /** Until when this is the answer, and until when it is still an answer. */
        final long freshUntil;

        final long usableUntil;

        CachedTextures(Map<String, Texture> textures, long savedAt, long freshUntil, long usableUntil) {
            this.textures = textures;
            this.savedAt = savedAt;
            this.freshUntil = freshUntil;
            this.usableUntil = usableUntil;
        }
    }
}

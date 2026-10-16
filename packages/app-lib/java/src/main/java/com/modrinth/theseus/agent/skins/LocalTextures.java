package com.modrinth.theseus.agent.skins;

import com.sun.net.httpserver.HttpServer;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Hands a file on disk to the game the only way the game will take one: over HTTP.
 *
 * <p>Textures reach it as a URL, and every version opens that URL as an {@code HttpURLConnection} —
 * a {@code file:} one does not even cast. So a file that is meant to be a skin is served from a
 * socket on the loopback address, which is what every launcher that has had to solve this does,
 * this one's own Ely.by agent included.
 *
 * <p>Each file is served under the digest of what is in it. Replacing a skin therefore changes its
 * URL, which is what makes the game fetch it again rather than reuse the copy it cached under the
 * old name.
 *
 * <p>Textures found anywhere else are relayed through here too ({@link #relay}). The game fetches
 * a texture once, with no retry and without the launcher's proxy, and a single timeout leaves the
 * player as Steve until the next session: that is how a licensed player came to see everyone's skin
 * but their own, the one on {@code textures.minecraft.net}, a CDN that is not always reachable.
 * Fetched here, the texture goes through the proxy, gets a few tries over both schemes, starts
 * downloading the moment it is known about and is kept for the session.
 */
final class LocalTextures {
    private static final Map<String, Path> SERVED = new ConcurrentHashMap<>();

    /** Relayed textures by key, and where each one really is. */
    private static final Map<String, String> RELAYED = new ConcurrentHashMap<>();

    /** Their bytes, once on the way. */
    private static final Map<String, CompletableFuture<byte[]>> FETCHED = new ConcurrentHashMap<>();

    /** A bound on what is kept, so a busy server cannot grow it without end. */
    private static final int MAX_FETCHED = 512;

    /** How long the game is kept waiting on a texture that is still on its way. */
    private static final long SERVE_WAIT_SECONDS = 25;

    private static final int ATTEMPTS = 3;

    private static final ExecutorService POOL = Executors.newCachedThreadPool(runnable -> {
        final Thread thread = new Thread(runnable, "noctrinth-skins-relay");
        thread.setDaemon(true);
        return thread;
    });

    private static HttpServer server;
    private static String origin;

    private LocalTextures() {}

    /** The URL this file can be fetched from, or null if it cannot be served at all. */
    static synchronized String publish(Path file) throws Exception {
        final String digest = digest(file);
        SERVED.put(digest, file);

        if (server == null && !start()) {
            return null;
        }

        return origin + "/" + digest + ".png";
    }

    /**
     * Where the game should fetch this texture from: through here, or the URL as it is when that
     * cannot be done. Starts fetching it at once, so it is usually ready before the game asks.
     */
    static synchronized String relay(String url) {
        if (url == null || !(url.startsWith("http://") || url.startsWith("https://")) || isOurs(url)) {
            return url;
        }
        if (server == null && !start()) {
            return url;
        }

        // Named by where it is, so the game's own cache of skins on disk, which goes
        // by this name, keeps finding it from one session to the next.
        final String key = "r" + hex(sha1(url.getBytes(StandardCharsets.UTF_8)));
        RELAYED.putIfAbsent(key, url);
        fetch(key);
        return origin + "/" + key + ".png";
    }

    /** Where a relayed URL really points; any other URL as it is. */
    static String sourceOf(String url) {
        if (!isOurs(url)) {
            return url;
        }
        final String name = url.substring(origin.length()).replace("/", "");
        final String key = name.endsWith(".png") ? name.substring(0, name.length() - 4) : name;
        final String source = RELAYED.get(key);
        return source == null ? url : source;
    }

    private static boolean isOurs(String url) {
        return origin != null && url.startsWith(origin);
    }

    private static CompletableFuture<byte[]> fetch(String key) {
        final String url = RELAYED.get(key);
        if (url == null) {
            return CompletableFuture.completedFuture(null);
        }
        if (FETCHED.size() >= MAX_FETCHED) {
            FETCHED.values().removeIf(CompletableFuture::isDone);
        }

        return FETCHED.computeIfAbsent(key, ignored -> {
            final CompletableFuture<byte[]> bytes = CompletableFuture.supplyAsync(() -> download(url), POOL);
            // A texture that could not be had is asked for again next time, not remembered as missing.
            bytes.whenComplete((result, failure) -> {
                if (result == null) {
                    FETCHED.remove(key);
                }
            });
            return bytes;
        });
    }

    /** The texture, tried a few times over both schemes; null when it is not to be had. */
    private static byte[] download(String url) {
        final String other = url.startsWith("http://")
                ? "https://" + url.substring("http://".length())
                : "http://" + url.substring("https://".length());

        for (int attempt = 0; attempt < ATTEMPTS; attempt++) {
            final Race race = race(url, other);
            if (race.bytes != null || race.notThere) {
                return race.bytes;
            }
            try {
                Thread.sleep(500L * (attempt + 1));
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                return null;
            }
        }
        return null;
    }

    /** How a race went: what was fetched, or whether somewhere said there is nothing there. */
    private static final class Race {
        volatile byte[] bytes;
        volatile boolean notThere;
    }

    /**
     * Both schemes at once, the first to deliver winning. One way in can hang until it times out
     * while the other answers at once, and asking them one after the other used to make every
     * texture wait out the slow one first.
     */
    private static Race race(String... urls) {
        final Race race = new Race();
        final CompletableFuture<Void> done = new CompletableFuture<>();
        final AtomicInteger pending = new AtomicInteger(urls.length);

        for (final String url : urls) {
            final Runnable attempt = () -> {
                try {
                    final byte[] bytes = Http.getBytes(url);
                    if (bytes != null) {
                        race.bytes = bytes;
                        done.complete(null);
                    } else {
                        // A 404: asked and told there is nothing there.
                        race.notThere = true;
                    }
                } catch (Throwable t) {
                    SkinSource.debug("Failed to fetch " + url + ": " + t);
                } finally {
                    if (pending.decrementAndGet() == 0) {
                        done.complete(null);
                    }
                }
            };
            try {
                POOL.execute(attempt);
            } catch (Throwable rejected) {
                attempt.run();
            }
        }

        try {
            done.get();
        } catch (Throwable t) {
            SkinSource.debug("Failed waiting on a texture: " + t);
        }
        return race;
    }

    private static byte[] relayed(String key) {
        try {
            return fetch(key).get(SERVE_WAIT_SECONDS, TimeUnit.SECONDS);
        } catch (Throwable t) {
            SkinSource.debug("Gave up waiting on a relayed texture: " + t);
            return null;
        }
    }

    private static boolean start() {
        try {
            server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
            // One texture still on its way must not hold up the others.
            server.setExecutor(POOL);
            server.createContext("/", exchange -> {
                try {
                    final String name = exchange.getRequestURI().getPath().replace("/", "");
                    final String key = name.endsWith(".png") ? name.substring(0, name.length() - 4) : name;
                    final Path file = SERVED.get(key);
                    final byte[] body =
                            file != null ? Files.readAllBytes(file) : RELAYED.containsKey(key) ? relayed(key) : null;

                    if (body == null) {
                        exchange.sendResponseHeaders(404, -1);
                        return;
                    }

                    exchange.getResponseHeaders().set("Content-Type", "image/png");
                    exchange.sendResponseHeaders(200, body.length);
                    try (OutputStream out = exchange.getResponseBody()) {
                        out.write(body);
                    }
                } catch (Throwable t) {
                    SkinSource.debug("Failed to serve a local skin: " + t);
                } finally {
                    exchange.close();
                }
            });

            // Its own thread would otherwise keep the game from ever exiting.
            Runtime.getRuntime().addShutdownHook(new Thread(() -> server.stop(0), "noctrinth-skins-stop"));

            server.start();
            origin = "http://" + server.getAddress().getAddress().getHostAddress() + ":"
                    + server.getAddress().getPort();
            SkinSource.debug("Serving skins from disk at " + origin);
            return true;
        } catch (Throwable t) {
            // No server, no local skins; everything else carries on.
            SkinSource.debug("Could not serve skins from disk: " + t);
            server = null;
            return false;
        }
    }

    /** What is in the file, as something that can go in a URL. */
    private static String digest(Path file) throws Exception {
        return hex(sha1(Files.readAllBytes(file)));
    }

    private static byte[] sha1(byte[] data) {
        try {
            return MessageDigest.getInstance("SHA-1").digest(data);
        } catch (Exception e) {
            throw new IllegalStateException(e);
        }
    }

    private static String hex(byte[] hash) {
        final StringBuilder hex = new StringBuilder(hash.length * 2);
        for (final byte b : hash) {
            hex.append(Character.forDigit((b >> 4) & 0xf, 16));
            hex.append(Character.forDigit(b & 0xf, 16));
        }

        return hex.toString();
    }
}

package com.modrinth.theseus.agent.skins;

import com.sun.net.httpserver.HttpServer;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

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
 */
final class LocalTextures {
    private static final Map<String, Path> SERVED = new ConcurrentHashMap<>();

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

    private static boolean start() {
        try {
            server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
            server.createContext("/", exchange -> {
                try {
                    final String name = exchange.getRequestURI().getPath().replace("/", "");
                    final Path file = SERVED.get(name.endsWith(".png") ? name.substring(0, name.length() - 4) : name);
                    final byte[] body = file == null ? null : Files.readAllBytes(file);

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
        final byte[] hash = MessageDigest.getInstance("SHA-1").digest(Files.readAllBytes(file));

        final StringBuilder hex = new StringBuilder(hash.length * 2);
        for (final byte b : hash) {
            hex.append(Character.forDigit((b >> 4) & 0xf, 16));
            hex.append(Character.forDigit(b & 0xf, 16));
        }

        return hex.toString();
    }
}

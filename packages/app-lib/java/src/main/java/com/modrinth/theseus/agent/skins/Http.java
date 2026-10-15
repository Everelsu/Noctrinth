package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonParser;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.net.HttpURLConnection;
import java.net.InetSocketAddress;
import java.net.Proxy;
import java.net.URI;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.Charset;
import java.util.Locale;

/** The one way a skin source talks to the network: a GET, with an answer or nothing. */
final class Http {
    private static final int CONNECT_TIMEOUT_MS = 4000;
    private static final int READ_TIMEOUT_MS = 4000;

    /**
     * The proxy the launcher's own requests go through, for these to go through too.
     *
     * <p>Only these: the game's connections are left exactly as they were. A launcher that needs a
     * proxy to reach Mojang quickly is on a network where the lookups need one as well, and without
     * it a licensed player's skin arrives after the game has stopped waiting for it.
     */
    private static final Proxy PROXY = parseProxy(System.getProperty("noctrinth.skins.proxy"));

    private Http() {}

    /**
     * The JSON at a URL, or null when the source has never heard of whoever was asked about.
     *
     * <p>Ely.by says "no such player" with a 204 and Mojang with a 404, and neither has a body to
     * read. A source that is having trouble instead -- a 5xx, or a rate limit -- is telling us
     * something else entirely, and throws: that answer is worth nothing, and worth not asking for
     * again for a moment. See {@link SkinSource}.
     */
    static JsonElement getJson(String url) throws Exception {
        final HttpURLConnection connection = open(url);
        connection.setConnectTimeout(CONNECT_TIMEOUT_MS);
        connection.setReadTimeout(READ_TIMEOUT_MS);
        connection.setRequestProperty("Accept", "application/json");
        connection.setRequestProperty("User-Agent", "Noctrinth");

        try {
            final int status = status(connection, url);
            if (status == 429) {
                // Being rate limited is not being broken, and the two must not
                // be answered the same way. Mojang's name lookup is the tight
                // one, and on a local network every client behind the same
                // address is spending the same allowance — so it is reached
                // exactly when several people are playing together, which is
                // the moment their skins are wanted. Benching it then leaves
                // the licensed players as Steve for as long as it lasts, and
                // the source that could not answer for them anyway is the only
                // one left being asked.
                throw new RateLimited(url, retryAfterMs(connection));
            }
            if (isOutage(status)) {
                throw new Unreachable(url, new IOException("got " + status));
            }
            if (status >= 500) {
                throw new IOException("asked " + url + " and got " + status);
            }
            if (status != HttpURLConnection.HTTP_OK) {
                SkinSource.debug("Asked " + url + " and got " + status);
                return null;
            }

            try (InputStream stream = connection.getInputStream();
                    Reader reader = new InputStreamReader(stream, Charset.forName("UTF-8"))) {
                return JsonParser.parseReader(reader);
            }
        } finally {
            connection.disconnect();
        }
    }

    private static HttpURLConnection open(String url) throws IOException {
        final URL target = new URL(url);
        return (HttpURLConnection) (PROXY == null ? target.openConnection() : target.openConnection(PROXY));
    }

    /**
     * A proxy from the URL the launcher uses, or null for none or one this cannot speak.
     *
     * <p>Left unresolved, so that a SOCKS proxy is handed the name rather than an address looked up
     * here — on the networks that need one, the lookup is often what does not work without it.
     */
    static Proxy parseProxy(String configured) {
        if (configured == null || configured.trim().isEmpty()) {
            return null;
        }

        try {
            final URI uri = new URI(configured.trim());
            final String scheme = uri.getScheme() == null ? "" : uri.getScheme().toLowerCase(Locale.ROOT);
            final Proxy.Type type = scheme.startsWith("socks")
                    ? Proxy.Type.SOCKS
                    : "http".equals(scheme) || "https".equals(scheme) ? Proxy.Type.HTTP : null;
            if (type == null || uri.getHost() == null) {
                return null;
            }

            final int port = uri.getPort() > 0 ? uri.getPort() : type == Proxy.Type.SOCKS ? 1080 : 8080;
            return new Proxy(type, InetSocketAddress.createUnresolved(uri.getHost(), port));
        } catch (Exception malformed) {
            SkinSource.debug("Ignoring a proxy that is not a URL: " + configured);
            return null;
        }
    }

    /**
     * The status of a request, telling a source that cannot be reached at all apart from one that
     * could not answer this one question.
     *
     * <p>The difference decides whether the source is left out of the next lookups. A connection
     * that cannot be made is the source being down. A source that connects and then fails or takes
     * too long is often only failing for this name: Ely.by's proxy to Mojang answers a licensed
     * player with a 500, a 204 or nothing at all, and leaving Ely.by out for that would leave every
     * one of its own players as Steve.
     */
    private static int status(HttpURLConnection connection, String url) throws IOException {
        try {
            connection.connect();
        } catch (IOException down) {
            throw new Unreachable(url, down);
        }
        return connection.getResponseCode();
    }

    /** The statuses a gateway answers with when what is behind it is down, not just this name. */
    private static boolean isOutage(int status) {
        return status == 502 || status == 503 || status == 504;
    }

    /** A source that could not be reached at all, as opposed to one that failed one question. */
    static final class Unreachable extends IOException {
        private static final long serialVersionUID = 1L;

        Unreachable(String url, IOException cause) {
            super("could not reach " + url + ": " + cause.getMessage(), cause);
        }
    }

    /** How long the source asked to be left alone for, or a short guess. */
    private static long retryAfterMs(HttpURLConnection connection) {
        final String header = connection.getHeaderField("Retry-After");
        if (header != null) {
            try {
                return Math.max(1000L, Long.parseLong(header.trim()) * 1000L);
            } catch (NumberFormatException notSeconds) {
                // A date rather than a count of seconds, which is allowed and
                // is not worth parsing for this.
            }
        }

        return DEFAULT_RETRY_AFTER_MS;
    }

    /** What a source that would not say when asked is left alone for. */
    private static final long DEFAULT_RETRY_AFTER_MS = 10 * 1000L;

    /** A source that is well, busy, and said so. */
    static final class RateLimited extends IOException {
        private static final long serialVersionUID = 1L;

        final long retryAfterMs;

        RateLimited(String url, long retryAfterMs) {
            super("asked " + url + " and was told to wait " + retryAfterMs + "ms");
            this.retryAfterMs = retryAfterMs;
        }
    }

    /**
     * The bytes at a URL, or null when there is nothing there for whoever was asked about.
     *
     * <p>The same rules as {@link #getJson}: a 404 is an answer, a 5xx or a rate limit is not.
     */
    static byte[] getBytes(String url) throws Exception {
        final HttpURLConnection connection = open(url);
        connection.setConnectTimeout(CONNECT_TIMEOUT_MS);
        connection.setReadTimeout(READ_TIMEOUT_MS);
        connection.setRequestProperty("User-Agent", "Noctrinth");

        try {
            final int status = status(connection, url);
            if (status == 429) {
                throw new RateLimited(url, retryAfterMs(connection));
            }
            if (isOutage(status)) {
                throw new Unreachable(url, new IOException("got " + status));
            }
            if (status >= 500) {
                throw new IOException("asked " + url + " and got " + status);
            }
            if (status != HttpURLConnection.HTTP_OK) {
                SkinSource.debug("Asked " + url + " and got " + status);
                return null;
            }

            try (InputStream stream = connection.getInputStream()) {
                final ByteArrayOutputStream collected = new ByteArrayOutputStream();
                final byte[] buffer = new byte[8192];
                int read;
                long total = 0;
                while ((read = stream.read(buffer)) != -1) {
                    total += read;
                    if (total > MAX_TEXTURE_BYTES) {
                        throw new IOException(url + " is larger than a texture has any reason to be");
                    }
                    collected.write(buffer, 0, read);
                }
                return collected.toByteArray();
            }
        } finally {
            connection.disconnect();
        }
    }

    /** What a texture will not be bigger than, so a bad answer cannot be read forever. */
    private static final long MAX_TEXTURE_BYTES = 2 * 1024 * 1024;

    /** A name as it can be put in a path. */
    static String encode(String value) throws Exception {
        return URLEncoder.encode(value, "UTF-8");
    }
}

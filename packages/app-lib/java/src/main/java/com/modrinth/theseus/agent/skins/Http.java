package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonParser;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.Charset;

/** The one way a skin source talks to the network: a GET, with an answer or nothing. */
final class Http {
    private static final int CONNECT_TIMEOUT_MS = 4000;
    private static final int READ_TIMEOUT_MS = 4000;

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
        final HttpURLConnection connection = (HttpURLConnection) new URL(url).openConnection();
        connection.setConnectTimeout(CONNECT_TIMEOUT_MS);
        connection.setReadTimeout(READ_TIMEOUT_MS);
        connection.setRequestProperty("Accept", "application/json");
        connection.setRequestProperty("User-Agent", "Noctrinth");

        try {
            final int status = connection.getResponseCode();
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
        final HttpURLConnection connection = (HttpURLConnection) new URL(url).openConnection();
        connection.setConnectTimeout(CONNECT_TIMEOUT_MS);
        connection.setReadTimeout(READ_TIMEOUT_MS);
        connection.setRequestProperty("User-Agent", "Noctrinth");

        try {
            final int status = connection.getResponseCode();
            if (status == 429) {
                throw new RateLimited(url, retryAfterMs(connection));
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

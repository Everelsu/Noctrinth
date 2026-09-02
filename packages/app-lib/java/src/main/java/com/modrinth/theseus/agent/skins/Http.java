package com.modrinth.theseus.agent.skins;

import com.google.gson.JsonElement;
import com.google.gson.JsonParser;
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
            if (status >= 500 || status == 429) {
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

    /** A name as it can be put in a path. */
    static String encode(String value) throws Exception {
        return URLEncoder.encode(value, "UTF-8");
    }
}

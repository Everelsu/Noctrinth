package com.modrinth.theseus.agent.skins;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * Skins from the folder the launcher makes, and the loopback server that hands them to the game.
 *
 * <p>The game only knows how to fetch a texture over HTTP, so what is on disk has to be served
 * rather than pointed at — and served under a name that changes when the file does, or the game
 * would go on using the copy it cached.
 */
class LocalSourceTest {
    @Test
    void servesASkinPutThereByHand(@TempDir Path folder) throws Exception {
        final byte[] png = "not really a png, but bytes all the same".getBytes("UTF-8");
        Files.write(folder.resolve("Offline_Friend.png"), png);

        final Map<String, SkinSource.Texture> textures = new LocalSource(folder).textures("Offline_Friend");
        final SkinSource.Texture skin = textures.get("SKIN");

        assertNotNull(skin, "the file should have been found");
        assertTrue(skin.url.startsWith("http://127.0.0.1:"), "it has to be reachable over HTTP: " + skin.url);
        assertArrayEquals(png, fetch(skin.url), "and it should serve exactly what is in the file");
    }

    @Test
    void findsTheFileWhateverCaseItWasNamedIn(@TempDir Path folder) throws Exception {
        Files.write(folder.resolve("someONE.png"), new byte[] {1, 2, 3});

        assertNotNull(new LocalSource(folder).textures("Someone").get("SKIN"));
    }

    @Test
    void saysSoWhenTheSkinIsForTheSlimModel(@TempDir Path folder) throws Exception {
        Files.write(folder.resolve("Thin_One-slim.png"), new byte[] {1, 2, 3});

        final SkinSource.Texture skin =
                new LocalSource(folder).textures("Thin_One").get("SKIN");

        assertNotNull(skin);
        assertEquals("slim", skin.metadata.get("model"));
    }

    @Test
    void hasNothingForANameNobodyLeftAFileFor(@TempDir Path folder) throws Exception {
        assertTrue(new LocalSource(folder).textures("Nobody").isEmpty());
    }

    @Test
    void picksUpCapesFromBesideTheSkins(@TempDir Path folder) throws Exception {
        Files.createDirectories(folder.resolve("capes"));
        Files.write(folder.resolve("capes").resolve("Caped_One.png"), new byte[] {4, 5, 6});

        final Map<String, SkinSource.Texture> textures = new LocalSource(folder).textures("Caped_One");

        assertNotNull(textures.get("CAPE"), "a cape on its own should still be served");
        assertNull(textures.get("SKIN"), "and nothing should be invented for the skin");
    }

    /** Replacing the file has to change the URL, or the game will keep the copy it already has. */
    @Test
    void servesAChangedFileUnderANewName(@TempDir Path folder) throws Exception {
        final Path file = folder.resolve("Repainted.png");
        Files.write(file, new byte[] {1, 1, 1});
        final String before = new LocalSource(folder).textures("Repainted").get("SKIN").url;

        Files.write(file, new byte[] {2, 2, 2});
        final String after = new LocalSource(folder).textures("Repainted").get("SKIN").url;

        assertNotEquals(before, after, "a repainted skin should not answer to the old address");
        assertArrayEquals(new byte[] {2, 2, 2}, fetch(after), "and the new address should serve the new file");
    }

    private static byte[] fetch(String url) throws Exception {
        final HttpURLConnection connection = (HttpURLConnection) new URL(url).openConnection();
        connection.setConnectTimeout(2000);
        connection.setReadTimeout(2000);

        try {
            assertEquals(200, connection.getResponseCode(), "the loopback server should have answered");
            try (InputStream stream = connection.getInputStream()) {
                final ByteArrayOutputStream out = new ByteArrayOutputStream();
                final byte[] buffer = new byte[4096];
                int read;
                while ((read = stream.read(buffer)) != -1) {
                    out.write(buffer, 0, read);
                }
                return out.toByteArray();
            }
        } finally {
            connection.disconnect();
        }
    }
}

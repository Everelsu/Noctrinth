package com.modrinth.theseus.agent.skins;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.awt.image.BufferedImage;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.net.URL;
import java.nio.charset.Charset;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import javax.imageio.ImageIO;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * OptiFine's capes, which arrive in a shape the game cannot read.
 *
 * <p>What matters is the redraw: 46 by 22 is the top left corner of the 64 by 32 the game expects,
 * so handing the picture over as it came would put the cape half off the player. The rest is that a
 * cape-only source answers with a cape and nothing else, which is what the merged lookup needs.
 */
class OptiFineCapeSourceTest {
    private static ServerSocket capes;
    private static Thread server;

    @BeforeAll
    static void startCapeServer() throws Exception {
        capes = new ServerSocket(0, 0, InetAddress.getLoopbackAddress());
        server = new Thread(
                () -> {
                    while (!capes.isClosed()) {
                        try (Socket socket = capes.accept()) {
                            readRequest(socket.getInputStream());

                            final byte[] png = pngOf(optiFineCape());
                            final OutputStream out = socket.getOutputStream();
                            out.write(("HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: " + png.length
                                            + "\r\nConnection: close\r\n\r\n")
                                    .getBytes(Charset.forName("UTF-8")));
                            out.write(png);
                            out.flush();
                        } catch (IOException closed) {
                            return;
                        } catch (Exception ignored) {
                            // The next request is its own business.
                        }
                    }
                },
                "optifine-cape-test");
        server.setDaemon(true);
        server.start();
    }

    @AfterAll
    static void stopCapeServer() throws Exception {
        capes.close();
        server.join(2000);
    }

    @Test
    void a_cape_is_redrawn_at_the_size_the_game_reads(@TempDir Path directory) throws Exception {
        final OptiFineCapeSource source = new OptiFineCapeSource(baseUrl(), directory);

        final Map<String, SkinSource.Texture> textures = source.textures("Notch");

        assertEquals(1, textures.size(), "a cape source answers with a cape and nothing else");
        final SkinSource.Texture cape = textures.get("CAPE");
        assertNotNull(cape, "the cape should be there");

        final BufferedImage served = ImageIO.read(new URL(cape.url));
        assertNotNull(served, "what was published should be a picture");
        assertEquals(64, served.getWidth());
        assertEquals(32, served.getHeight());

        // The original sits in the corner the game reads it from, and what was
        // never part of it stays empty rather than becoming black.
        assertEquals(0xFFFF0000, served.getRGB(0, 0), "the cape's own corner");
        assertEquals(0xFFFF0000, served.getRGB(45, 21), "the cape's far corner");
        assertEquals(0, served.getRGB(63, 31) >>> 24, "the canvas beyond it is transparent");
    }

    @Test
    void the_shape_is_recognised_at_every_scale() {
        assertTrue(OptiFineCapeSource.isOptiFineShape(46, 22));
        assertTrue(OptiFineCapeSource.isOptiFineShape(92, 44));
        assertTrue(OptiFineCapeSource.isOptiFineShape(184, 88));

        // What the game's own capes are, which are not to be redrawn.
        assertTrue(!OptiFineCapeSource.isOptiFineShape(64, 32));
        assertTrue(!OptiFineCapeSource.isOptiFineShape(512, 256));
    }

    @Test
    void nothing_there_is_no_cape(@TempDir Path directory) throws Exception {
        // A name nobody has a cape for: the source says so rather than
        // publishing an empty file for the game to fail on.
        final OptiFineCapeSource source = new OptiFineCapeSource("http://127.0.0.1:1/capes/", directory);

        try {
            assertTrue(source.textures("Nobody").isEmpty());
        } catch (Exception unreachable) {
            // Refused outright is the same answer as far as this is concerned,
            // and is what the chain treats as "this source could not say".
        }

        assertTrue(!Files.exists(directory.resolve("nobody.png")));
    }

    private static String baseUrl() {
        return "http://" + capes.getInetAddress().getHostAddress() + ":" + capes.getLocalPort() + "/capes/";
    }

    /** A cape in OptiFine's shape, filled so every pixel of it can be told from the canvas. */
    private static BufferedImage optiFineCape() {
        final BufferedImage cape = new BufferedImage(46, 22, BufferedImage.TYPE_INT_ARGB);
        for (int x = 0; x < cape.getWidth(); x++) {
            for (int y = 0; y < cape.getHeight(); y++) {
                cape.setRGB(x, y, 0xFFFF0000);
            }
        }
        return cape;
    }

    private static byte[] pngOf(BufferedImage image) throws Exception {
        final ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        ImageIO.write(image, "PNG", bytes);
        return bytes.toByteArray();
    }

    private static void readRequest(InputStream stream) throws IOException {
        final StringBuilder request = new StringBuilder();
        int read;
        while ((read = stream.read()) != -1) {
            request.append((char) read);
            if (request.length() >= 4 && request.indexOf("\r\n\r\n", request.length() - 4) >= 0) {
                return;
            }
        }
    }
}

package com.modrinth.theseus.agent.skins;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.Charset;
import java.util.Base64;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

/**
 * Mojang's two-step answer, against a stand-in for both ends of it.
 *
 * <p>A name buys an id and an id buys a profile, and the textures are packed inside that profile
 * the same way a server would have sent them.
 */
class MojangSourceTest {
    private static final Charset UTF_8 = Charset.forName("UTF-8");

    private static final String ID = "069a79f444e94726a5befca90e38aaf5";
    private static final String SKIN = "http://textures.example/mojang.png";

    private static final Map<String, String> ROUTES = new HashMap<>();

    private static ServerSocket server;
    private static Thread serverThread;
    private static MojangSource source;

    @BeforeAll
    static void startStubMojang() throws Exception {
        final String payload = "{\"profileName\":\"Notch\",\"textures\":{\"SKIN\":{\"url\":\"" + SKIN
                + "\",\"metadata\":{\"model\":\"slim\"}}}}";
        final String packed = Base64.getEncoder().encodeToString(payload.getBytes(UTF_8));

        ROUTES.put("/users/profiles/minecraft/Notch", "{\"id\":\"" + ID + "\",\"name\":\"Notch\"}");
        ROUTES.put(
                "/session/minecraft/profile/" + ID,
                "{\"id\":\"" + ID + "\",\"name\":\"Notch\",\"properties\":[{\"name\":\"signature\",\"value\":\"…\"},"
                        + "{\"name\":\"textures\",\"value\":\"" + packed + "\"}]}");

        server = new ServerSocket(0, 0, InetAddress.getByName("127.0.0.1"));
        serverThread = new Thread(MojangSourceTest::serve, "stub-mojang");
        serverThread.setDaemon(true);
        serverThread.start();

        final String base = "http://127.0.0.1:" + server.getLocalPort();
        source = new MojangSource(base + "/users/profiles/minecraft/", base + "/session/minecraft/profile/");
    }

    @AfterAll
    static void stopStubMojang() throws Exception {
        server.close();
        serverThread.join(2000);
    }

    @Test
    void unpacksTheTexturesOnTheProfile() throws Exception {
        final Map<String, SkinSource.Texture> textures = source.textures("Notch");
        final SkinSource.Texture skin = textures.get("SKIN");

        assertNotNull(skin, "the profile carried a skin");
        assertEquals(SKIN, skin.url);
        assertEquals("slim", skin.metadata.get("model"), "slim arms should survive the round trip");
    }

    @Test
    void hasNothingForANameMojangDoesNotKnow() throws Exception {
        assertTrue(source.textures("Nobody").isEmpty(), "an unknown name should not resolve to anything");
    }

    @Test
    void isWhatTheWordMojangMeansInTheSourceList() {
        final List<SkinSource.Source> sources = SkinSource.parseSources("https://skinsystem.example/, mojang");

        assertEquals(2, sources.size());
        assertEquals("https://skinsystem.example", sources.get(0).toString(), "a trailing slash should be trimmed");
        assertEquals(MojangSource.NAME, sources.get(1).toString());
    }

    /** Answers what it has a route for, and 404 for everything else. */
    private static void serve() {
        while (!server.isClosed()) {
            try (Socket socket = server.accept()) {
                final StringBuilder requestLine = new StringBuilder();
                final InputStream in = socket.getInputStream();
                int c;
                while ((c = in.read()) != -1 && c != '\n') {
                    if (c != '\r') {
                        requestLine.append((char) c);
                    }
                }

                final String[] parts = requestLine.toString().split(" ");
                final String body = parts.length >= 2 ? ROUTES.get(parts[1]) : null;
                final OutputStream out = socket.getOutputStream();

                if (body == null) {
                    out.write("HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n".getBytes(UTF_8));
                    out.flush();
                    continue;
                }

                final byte[] bytes = body.getBytes(UTF_8);
                out.write(("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: " + bytes.length
                                + "\r\nConnection: close\r\n\r\n")
                        .getBytes(UTF_8));
                out.write(bytes);
                out.flush();
            } catch (IOException e) {
                // The socket closing is how this thread is asked to stop.
                return;
            }
        }
    }
}

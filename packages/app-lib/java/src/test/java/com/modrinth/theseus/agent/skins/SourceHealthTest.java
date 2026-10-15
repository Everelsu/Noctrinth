package com.modrinth.theseus.agent.skins;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.JsonParser;
import java.awt.image.BufferedImage;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.Map;
import org.junit.jupiter.api.Test;

/**
 * Which failures leave a source out of the next lookups, and how capes from other services are put
 * in the game's shape.
 *
 * <p>Ely.by's proxy to Mojang answers a licensed player with a 500 or not at all. That used to leave
 * Ely.by out for a minute, and every one of its own players was Steve for it.
 */
class SourceHealthTest {
    @Test
    void a_source_that_fails_one_name_is_still_asked_about_the_next() throws Exception {
        try (ServerSocket server = answering("500 Internal Server Error")) {
            final SkinSource.Source source = new SkinSystemSource(origin(server));

            assertNull(SkinSource.ask(source, "Notch"), "a failure is not an answer");
            assertFalse(SkinSource.isBenched(source), "a 500 for one name is not the source being down");
        }
    }

    @Test
    void a_source_that_cannot_be_reached_is_left_out() throws Exception {
        final int port;
        try (ServerSocket closed = new ServerSocket(0, 1, InetAddress.getLoopbackAddress())) {
            port = closed.getLocalPort();
        }
        final SkinSource.Source source = new SkinSystemSource("http://127.0.0.1:" + port);

        assertThrows(Http.Unreachable.class, () -> source.textures("Everesu"));
        assertNull(SkinSource.ask(source, "Everesu"));
        assertTrue(SkinSource.isBenched(source), "nothing listening is the source being down");
    }

    @Test
    void a_gateway_with_nothing_behind_it_is_the_source_being_down() throws Exception {
        try (ServerSocket server = answering("503 Service Unavailable")) {
            final SkinSource.Source source = new SkinSystemSource(origin(server));

            assertThrows(Http.Unreachable.class, () -> source.textures("Everesu"));
        }
    }

    @Test
    void a_lookup_that_could_not_finish_keeps_the_skin_it_had() {
        final Map<String, SkinSource.Texture> skin = java.util.Collections.singletonMap(
                "SKIN", new SkinSource.Texture("http://skins.example/a.png", java.util.Collections.emptyMap()));
        SkinSource.store("Keeper", skin, System.currentTimeMillis(), true);

        SkinSource.store("Keeper", java.util.Collections.emptyMap(), System.currentTimeMillis(), false);
        assertEquals(
                "http://skins.example/a.png", SkinSource.remembered("Keeper").get("SKIN").url);

        // A lookup that heard from everyone is believed, skin gone or not.
        SkinSource.store("Keeper", java.util.Collections.emptyMap(), System.currentTimeMillis(), true);
        assertTrue(SkinSource.remembered("Keeper").isEmpty());
    }

    @Test
    void every_service_cape_ends_up_the_shape_the_game_reads() {
        // OptiFine's cut-down canvas, LabyMod's cape alone, and an animated one.
        assertSize(64, 32, CapeImages.onVanillaCanvas(image(46, 22)));
        assertSize(128, 64, CapeImages.onVanillaCanvas(image(92, 44)));
        assertSize(1024, 512, CapeImages.onVanillaCanvas(image(352, 272)));
        assertSize(64, 32, CapeImages.onVanillaCanvas(image(64, 128)));
        assertSize(64, 32, CapeImages.onVanillaCanvas(image(64, 32)));
    }

    @Test
    void a_minecraftcapes_profile_names_its_cape() {
        assertEquals(
                "https://textures.minecraftcapes.net/capes/a",
                CapeServiceSource.capeUrl(JsonParser.parseString(
                        "{\"cape_url\":\"https://textures.minecraftcapes.net/capes/a\",\"animated_cape_url\":null}")));
        assertEquals(
                "https://textures.minecraftcapes.net/capes/b",
                CapeServiceSource.capeUrl(JsonParser.parseString(
                        "{\"cape_url\":null,\"animated_cape_url\":\"https://textures.minecraftcapes.net/capes/b\"}")));
        assertNull(CapeServiceSource.capeUrl(JsonParser.parseString("{\"cape_url\":null,\"animated_cape_url\":null}")));
    }

    @Test
    void the_launchers_proxy_is_understood() {
        final java.net.Proxy http = Http.parseProxy("http://127.0.0.1:10809");
        assertEquals(java.net.Proxy.Type.HTTP, http.type());
        assertEquals(
                "127.0.0.1:10809",
                ((java.net.InetSocketAddress) http.address()).getHostString() + ":"
                        + ((java.net.InetSocketAddress) http.address()).getPort());

        assertEquals(
                java.net.Proxy.Type.SOCKS,
                Http.parseProxy("socks5h://proxy.example:1080").type());
        assertNull(Http.parseProxy(""));
        assertNull(Http.parseProxy("ftp://proxy.example:21"));
        assertNull(Http.parseProxy("not a url at all"));
    }

    @Test
    void a_littleskin_profile_names_its_skin_and_cape() {
        final Map<String, SkinSource.Texture> classic = CustomSkinApiSource.readProfile(
                JsonParser.parseString("{\"skins\":{\"default\":\"aa\"},\"cape\":\"cc\"}"), "https://s");
        assertEquals("https://s/textures/aa", classic.get("SKIN").url);
        assertTrue(classic.get("SKIN").metadata.isEmpty());
        assertEquals("https://s/textures/cc", classic.get("CAPE").url);

        final Map<String, SkinSource.Texture> slim = CustomSkinApiSource.readProfile(
                JsonParser.parseString("{\"skins\":{\"slim\":\"bb\",\"default\":\"aa\"},\"cape\":null}"), "https://s");
        assertEquals("https://s/textures/bb", slim.get("SKIN").url);
        assertEquals("slim", slim.get("SKIN").metadata.get("model"));
        assertFalse(slim.containsKey("CAPE"));

        assertTrue(CustomSkinApiSource.readProfile(JsonParser.parseString("{}"), "https://s")
                .isEmpty());
        assertTrue(CustomSkinApiSource.readProfile(
                        JsonParser.parseString("{\"skins\":{\"default\":null},\"cape\":null}"), "https://s")
                .isEmpty());
    }

    @Test
    void an_id_gets_its_dashes() {
        assertEquals(
                "069a79f4-44e9-4726-a5be-fca90e38aaf5", CapeServiceSource.dashed("069a79f444e94726a5befca90e38aaf5"));
    }

    private static void assertSize(int width, int height, BufferedImage image) {
        assertEquals(width, image.getWidth());
        assertEquals(height, image.getHeight());
    }

    private static BufferedImage image(int width, int height) {
        return new BufferedImage(width, height, BufferedImage.TYPE_INT_ARGB);
    }

    /** A server that gives every request the same empty answer with this status line. */
    private static ServerSocket answering(String status) throws IOException {
        final ServerSocket server = new ServerSocket(0, 8, InetAddress.getLoopbackAddress());
        final Thread thread = new Thread(
                () -> {
                    while (!server.isClosed()) {
                        try (Socket socket = server.accept()) {
                            socket.getInputStream().read(new byte[4096]);
                            final OutputStream out = socket.getOutputStream();
                            out.write(("HTTP/1.1 " + status + "\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                                    .getBytes(StandardCharsets.UTF_8));
                            out.flush();
                        } catch (IOException closed) {
                            return;
                        }
                    }
                },
                "source-health-test");
        thread.setDaemon(true);
        thread.start();
        return server;
    }

    private static String origin(ServerSocket server) {
        return "http://127.0.0.1:" + server.getLocalPort();
    }
}
